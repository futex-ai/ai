//! xAI thinking-level request mapping tests.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Model, ModelRequest};
use ai_models_core::ThinkingLevel;
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use super::{XaiModel, test_support::recording_http_client};

#[tokio::test]
async fn builds_xai_thinking_variant_requests() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Done",
                    "tool_calls": []
                }
            }]
        }),
    });
    let model = XaiModel::with_catalog_auth(
        http_client,
        "grok-4.5-thinking-high",
        "grok-4.5",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("xAI thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["model"], "grok-4.5");
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("grok-4.5-thinking-high")
    );
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
    assert_eq!(response.model_id, "grok-4.5");
}

#[tokio::test]
async fn downgrades_max_to_grok_high() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "Done", "tool_calls": [] }
            }]
        }),
    });
    let model = XaiModel::with_catalog_auth(
        http_client,
        "custom-grok-4.5-max",
        "grok-4.5",
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("xAI downgraded response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
}

#[tokio::test]
async fn omits_reasoning_effort_for_grok_4_20() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Done",
                    "tool_calls": []
                }
            }]
        }),
    });
    let model = XaiModel::with_catalog_auth(
        http_client,
        "grok-4.20-reasoning",
        "grok-4.20-reasoning",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    model
        .complete(&simple_request())
        .await
        .expect("xAI reasoning response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["model"], "grok-4.20-reasoning");
    assert!(body.get("reasoning_effort").is_none());
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
