//! OpenAI thinking-level request mapping tests.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Model, ModelRequest, ToolDefinition};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{GPT_5_5, GPT_5_6_SOL, GPT_5_6_SOL_THINKING_MAX};

use super::OpenAiModel;
use crate::openai::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn builds_openai_thinking_variant_requests() {
    let (http_client, requests) = client_for_buffered_bodies(vec![json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "Done" }]
        }]
    })]);
    let model = OpenAiModel::with_catalog_auth(
        http_client,
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        ThinkingLevel::ExtraHigh,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("OpenAI thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(requests[0].url, "https://api.openai.com/v1/responses");
    assert_eq!(body["model"], "gpt-5.5");
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    assert_eq!(body["include"][0], "reasoning.encrypted_content");
    assert_eq!(body["tools"][0]["name"], "memory_read");
    assert_eq!(body["tool_choice"], "auto");
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("gpt-5.5-thinking-extra-high")
    );
    assert_eq!(response.thinking_level.as_deref(), Some("extra_high"));
    assert_eq!(response.model_id, "gpt-5.5");
}

#[tokio::test]
async fn maps_openai_max_thinking_to_max_effort() {
    let (http_client, requests) = client_for_buffered_bodies(vec![json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "Done" }]
        }]
    })]);
    let model = OpenAiModel::with_catalog_auth(
        http_client,
        GPT_5_6_SOL_THINKING_MAX,
        GPT_5_6_SOL,
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("OpenAI max-thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["model"], GPT_5_6_SOL);
    assert_eq!(body["reasoning"]["effort"], "max");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some(GPT_5_6_SOL_THINKING_MAX)
    );
    assert_eq!(response.thinking_level.as_deref(), Some("max"));
}

#[tokio::test]
async fn downgrades_max_to_gpt_5_5_extra_high() {
    let (http_client, requests) = client_for_buffered_bodies(vec![json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "Done" }]
        }]
    })]);
    let model = OpenAiModel::with_catalog_auth(
        http_client,
        "custom-gpt-5.5-max",
        GPT_5_5,
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("OpenAI downgraded response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    assert_eq!(response.thinking_level.as_deref(), Some("extra_high"));
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: vec![ToolDefinition {
            name: "memory_read".to_owned(),
            description: "Read memory".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
            activity_verb: None,
        }],
        response_schema: None,
        controls: Default::default(),
    }
}
