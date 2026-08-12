//! Anthropic prompt-cache request tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, ModelRequest, StructuredOutputSchema, ToolDefinition};
use ai_models_core::ThinkingLevel;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, DynJsonHttpTransport, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::Unimock;

use super::{
    AnthropicModel,
    cache::{AnthropicCacheTtl, AnthropicPromptCache},
    request::build_request,
};

#[test]
fn system_serializes_as_a_text_block_array_with_structured_output_suffix() {
    let request = ModelRequest {
        system_prompt: "Follow policy.".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: Some(StructuredOutputSchema {
            name: "status".to_owned(),
            schema: json!({
                "type": "object",
                "properties": {"done": {"type": "boolean"}}
            }),
        }),
    };

    let body = serialized_request(five_minute_cache(), &request);
    let system = body["system"]
        .as_array()
        .expect("system should be a block array");
    assert_eq!(system.len(), 1);
    assert_eq!(system[0]["type"], "text");
    let text = system[0]["text"]
        .as_str()
        .expect("system text should be a string");
    assert!(text.starts_with("Follow policy."));
    assert!(text.contains("return raw JSON only"));
    assert!(text.contains("schema `status`"));
}

#[test]
fn empty_effective_system_prompt_is_omitted() {
    let request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
    };

    let body = serialized_request(five_minute_cache(), &request);

    assert!(body.get("system").is_none());
}

#[test]
fn every_constructor_defaults_to_five_minute_caching() {
    let http_client = unused_http_client();
    let auth = static_auth();
    let models = [
        AnthropicModel::new(http_client.clone(), "model", "key"),
        AnthropicModel::with_auth(http_client.clone(), "model", auth.clone()),
        AnthropicModel::with_catalog_auth(
            http_client,
            "catalog-model",
            "provider-model",
            ThinkingLevel::Disabled,
            auth,
        ),
    ];

    for model in models {
        assert_eq!(model.prompt_cache, five_minute_cache());
    }
}

#[test]
fn disabled_caching_omits_cache_control_everywhere() {
    let model = AnthropicModel::new(unused_http_client(), "model", "key")
        .with_prompt_cache(AnthropicPromptCache::Disabled);
    let body = serialized_request(
        model.prompt_cache,
        &ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("hello")],
            tools: vec![tool("inspect")],
            response_schema: None,
        },
    );

    assert_eq!(cache_control_count(&body), 0);
}

#[test]
fn configured_ttl_controls_cache_control_serialization() {
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
    };
    let five_minutes = serialized_request(five_minute_cache(), &request);
    let one_hour = serialized_request(
        AnthropicPromptCache::Enabled {
            ttl: AnthropicCacheTtl::OneHour,
        },
        &request,
    );

    assert_eq!(
        five_minutes["system"][0]["cache_control"],
        json!({"type": "ephemeral"})
    );
    assert_eq!(
        one_hour["system"][0]["cache_control"],
        json!({"type": "ephemeral", "ttl": "1h"})
    );
}

#[test]
fn non_empty_system_places_prefix_marker_on_system() {
    let body = serialized_request(
        five_minute_cache(),
        &ModelRequest {
            system_prompt: "system".to_owned(),
            messages: Vec::new(),
            tools: vec![tool("first"), tool("last")],
            response_schema: None,
        },
    );

    assert_eq!(
        body["system"][0]["cache_control"],
        json!({"type": "ephemeral"})
    );
    assert!(body["tools"][1].get("cache_control").is_none());
}

#[test]
fn blank_system_places_prefix_marker_on_last_tool() {
    let body = serialized_request(
        five_minute_cache(),
        &ModelRequest {
            system_prompt: " \n\t ".to_owned(),
            messages: Vec::new(),
            tools: vec![tool("first"), tool("last")],
            response_schema: None,
        },
    );

    assert!(body["system"][0].get("cache_control").is_none());
    assert_eq!(body["system"][0]["text"], " \n\t ");
    assert_eq!(
        body["tools"][1]["cache_control"],
        json!({"type": "ephemeral"})
    );
}

#[test]
fn blank_system_without_tools_emits_no_prefix_marker() {
    let body = serialized_request(
        five_minute_cache(),
        &ModelRequest {
            system_prompt: " \n\t ".to_owned(),
            messages: Vec::new(),
            tools: Vec::new(),
            response_schema: None,
        },
    );

    assert_eq!(body["system"][0]["text"], " \n\t ");
    assert_eq!(cache_control_count(&body), 0);
}

#[test]
fn empty_system_places_prefix_marker_on_last_tool() {
    let body = serialized_request(
        five_minute_cache(),
        &ModelRequest {
            system_prompt: String::new(),
            messages: Vec::new(),
            tools: vec![tool("first"), tool("last")],
            response_schema: None,
        },
    );

    assert!(body["tools"][0].get("cache_control").is_none());
    assert_eq!(
        body["tools"][1]["cache_control"],
        json!({"type": "ephemeral"})
    );
}

#[test]
fn empty_system_and_tools_emit_no_prefix_marker() {
    let body = serialized_request(
        five_minute_cache(),
        &ModelRequest {
            system_prompt: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            response_schema: None,
        },
    );

    assert_eq!(cache_control_count(&body), 0);
}

fn serialized_request(prompt_cache: AnthropicPromptCache, request: &ModelRequest) -> Value {
    serde_json::to_value(build_request(
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        request,
        prompt_cache,
    ))
    .expect("request should serialize")
}

fn five_minute_cache() -> AnthropicPromptCache {
    AnthropicPromptCache::Enabled {
        ttl: AnthropicCacheTtl::FiveMinutes,
    }
}

fn unused_http_client() -> DynJsonHttpClient {
    let transport: DynJsonHttpTransport = Arc::new(Unimock::new(()));
    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

fn static_auth() -> DynJsonHttpAuth {
    Arc::new(StaticHeaderAuth::new(BTreeMap::new()))
}

fn tool(name: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_owned(),
        description: format!("Use {name}"),
        input_schema: json!({"type": "object"}),
        activity_verb: None,
    }
}

fn cache_control_count(value: &Value) -> usize {
    match value {
        Value::Array(values) => values.iter().map(cache_control_count).sum(),
        Value::Object(values) => {
            usize::from(values.contains_key("cache_control"))
                + values.values().map(cache_control_count).sum::<usize>()
        }
        _ => 0,
    }
}
