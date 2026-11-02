//! xAI thinking-level request mapping tests.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use super::{GROK_4_5, GROK_4_20_REASONING, XaiModel, test_support::recording_http_client};

#[tokio::test]
async fn builds_xai_thinking_variant_requests() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
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
        GROK_4_5,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("xAI thinking response should parse");

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
    assert_eq!(body["model"], "grok-4.5");
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(response.catalog_model_id.as_deref(), Some("xai/grok-4.5"));
    assert_eq!(response.effort().map(Effort::as_str), Some("high"));
    assert_eq!(response.model_id, "grok-4.5");
}

#[tokio::test]
async fn downgrades_max_to_grok_high() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
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
        GROK_4_5,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("xAI downgraded response should parse");

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
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(response.effort().map(Effort::as_str), Some("high"));
}

#[tokio::test]
async fn omits_reasoning_effort_for_grok_4_20() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
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
        GROK_4_20_REASONING,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    model
        .complete(&simple_request())
        .await
        .expect("xAI reasoning response should parse");

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
