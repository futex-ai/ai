//! Anthropic thinking-level request mapping tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, Model, ModelRequest};
use ai_models_core::ThinkingLevel;
use json_http::{JsonHttpClient, JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use crate::CLAUDE_SONNET_5;
use crate::anthropic::stream_support::{RecordedRequests, client_for_buffered_bodies};

use super::AnthropicModel;

#[tokio::test]
async fn builds_anthropic_thinking_variant_requests_and_ignores_hidden_blocks() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
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
        "claude-opus-4-7-thinking-max",
        "claude-opus-4-7",
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
            "x-api-key".to_owned(),
            "anthropic-key".to_owned(),
        )]))),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("Anthropic thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["model"], "claude-opus-4-7");
    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["thinking"]["display"], "omitted");
    assert_eq!(body["output_config"]["effort"], "max");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("claude-opus-4-7-thinking-max")
    );
    assert_eq!(response.thinking_level.as_deref(), Some("max"));
    assert_eq!(response.model_id, "claude-opus-4-7");
    assert_eq!(response.assistant_message, "Done");
    assert!(!response.assistant_message.contains("hidden provider trace"));
}

#[tokio::test]
async fn downgrades_max_to_sonnet_high() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Done" }],
            "usage": { "input_tokens": 12, "output_tokens": 6 }
        }),
    });
    let model = AnthropicModel::with_catalog_auth(
        http_client,
        "custom-sonnet-max",
        CLAUDE_SONNET_5,
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::default()),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("Anthropic downgraded response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["output_config"]["effort"], "high");
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
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
