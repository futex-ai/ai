//! OpenAI thinking-level request mapping tests.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest, ToolDefinition};
use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::openai::stream_support::client_for_buffered_bodies;
use crate::{GPT_5_5, GPT_5_6_SOL};

use super::OpenAiModel;

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
        GPT_5_5,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::ExtraHigh);
    let response = model
        .complete(&request)
        .await
        .expect("OpenAI thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body");
    assert_eq!(requests[0].url, "https://api.openai.com/v1/responses");
    assert_eq!(body["model"], "gpt-5.5");
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    assert_eq!(body["include"][0], "reasoning.encrypted_content");
    assert_eq!(body["tools"][0]["name"], "memory_read");
    assert_eq!(body["tool_choice"], "auto");
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(response.catalog_model_id.as_deref(), Some("openai/gpt-5.5"));
    assert_eq!(response.effort().map(Effort::as_str), Some("extra_high"));
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
        GPT_5_6_SOL,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("OpenAI max-thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body");
    assert_eq!(body["model"], GPT_5_6_SOL.provider_model_id.as_ref());
    assert_eq!(body["reasoning"]["effort"], "max");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("openai/gpt-5.6-sol")
    );
    assert_eq!(response.effort().map(Effort::as_str), Some("max"));
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
        GPT_5_5,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("OpenAI downgraded response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body");
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    assert_eq!(response.effort().map(Effort::as_str), Some("extra_high"));
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
