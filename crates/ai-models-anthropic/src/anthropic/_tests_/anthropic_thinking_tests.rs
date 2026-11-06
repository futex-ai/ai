//! Anthropic thinking-level request mapping tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use json_http::{JsonHttpClient, JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use crate::anthropic::stream_support::{RecordedRequests, client_for_buffered_bodies};
use crate::{CLAUDE_OPUS_4_7, CLAUDE_SONNET_5};

use super::AnthropicModel;

#[tokio::test]
async fn builds_anthropic_thinking_variant_requests_and_ignores_hidden_blocks() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "hidden provider trace" },
                { "type": "text", "text": "Done" }
            ],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 6
            }
        }),
    });
    let model = AnthropicModel::with_catalog_auth(
        http_client,
        CLAUDE_OPUS_4_7,
        Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
            "x-api-key".to_owned(),
            "anthropic-key".to_owned(),
        )]))),
    );

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("Anthropic thinking response should parse");

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
    assert_eq!(body["model"], "claude-opus-4-7");
    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["thinking"]["display"], "omitted");
    assert_eq!(body["output_config"]["effort"], "max");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("anthropic/claude-opus-4-7")
    );
    assert_eq!(response.effort().map(Effort::as_str), Some("max"));
    assert_eq!(response.model_id, "claude-opus-4-7");
    assert_eq!(response.text(), "Done");
    assert!(!response.text().contains("hidden provider trace"));
}

#[tokio::test]
async fn downgrades_max_to_sonnet_high() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Done" }],
            "usage": { "input_tokens": 12, "output_tokens": 6 }
        }),
    });
    let model = AnthropicModel::with_catalog_auth(
        http_client,
        CLAUDE_SONNET_5,
        Arc::new(StaticHeaderAuth::default()),
    );

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("Anthropic downgraded response should parse");

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
    assert_eq!(body["output_config"]["effort"], "high");
    assert_eq!(response.effort().map(Effort::as_str), Some("high"));
}

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    client_for_buffered_bodies(vec![response.body])
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
