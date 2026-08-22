//! Anthropic streaming request and buffered-response parity tests.

use std::time::Duration;

use ai_interface::{ConversationMessage, Model, ModelRequest};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;
use serde_json::json;

use super::{AnthropicModel, response::parse_response};
use crate::anthropic::stream_support::{
    client_for_buffered_bodies, event, recording_streaming_client,
};

#[tokio::test]
async fn sends_streaming_request_with_completion_deadlines() {
    let body = text_response_body();
    let (http_client, requests) = client_for_buffered_bodies(vec![body]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");

    let requests = requests
        .lock()
        .expect("request lock should not be poisoned");
    let request = &requests[0];
    assert_eq!(request.body.as_ref().expect("request body")["stream"], true);
    assert_eq!(request.timeout, Duration::from_secs(3_600));
    assert_eq!(request.idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn explicit_total_timeout_overrides_streaming_default() {
    let (http_client, requests) = client_for_buffered_bodies(vec![text_response_body()]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");
    let mut request = simple_request();
    request.controls.execution.total_timeout = Some(Duration::from_secs(75));

    model
        .complete(&request)
        .await
        .expect("streamed response should parse");

    let requests = requests
        .lock()
        .expect("request lock should not be poisoned");
    assert_eq!(requests[0].timeout, Duration::from_secs(75));
    assert_eq!(requests[0].idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn streamed_deltas_match_the_buffered_response_mapper() {
    let buffered = json!({
        "stop_reason": "tool_use",
        "content": [
            {"type": "thinking", "thinking": "private thought", "signature": "signed"},
            {"type": "text", "text": "Done"},
            {
                "type": "tool_use",
                "id": "call_1",
                "name": "memory_read",
                "input": {"path": "root"}
            }
        ],
        "usage": {
            "input_tokens": 120,
            "cache_creation_input_tokens": 10,
            "cache_read_input_tokens": 40,
            "output_tokens": 32
        }
    });
    let events = vec![
        event(
            "future_event",
            json!({"type": "future_event", "value": "ignored"}),
        ),
        event(
            "message_start",
            json!({
                "type": "message_start",
                "message": {
                    "content": [],
                    "usage": {
                        "input_tokens": 120,
                        "cache_creation_input_tokens": 10,
                        "cache_read_input_tokens": 40,
                        "output_tokens": 1
                    }
                }
            }),
        ),
        event("ping", json!({"type": "ping"})),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "thinking", "thinking": "", "signature": ""}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "thinking_delta", "thinking": "private "}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "thinking_delta", "thinking": "thought"}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "signature_delta", "signature": "signed"}
            }),
        ),
        event(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 0}),
        ),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 1,
                "content_block": {"type": "text", "text": ""}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 1,
                "delta": {"type": "citations_delta", "citation": {}}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 1,
                "delta": {"type": "text_delta", "text": "Do"}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 1,
                "delta": {"type": "text_delta", "text": "ne"}
            }),
        ),
        event(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 1}),
        ),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 2,
                "content_block": {
                    "type": "tool_use",
                    "id": "call_1",
                    "name": "memory_read",
                    "input": {}
                }
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 2,
                "delta": {"type": "input_json_delta", "partial_json": "{\"path\":"}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 2,
                "delta": {"type": "input_json_delta", "partial_json": "\"root\"}"}
            }),
        ),
        event(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 2}),
        ),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 3,
                "content_block": {"type": "fallback"}
            }),
        ),
        event(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 3}),
        ),
        event(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {},
                "usage": {"output_tokens": 16}
            }),
        ),
        event(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {"stop_reason": "tool_use"},
                "usage": {"output_tokens": 32}
            }),
        ),
        event("message_stop", json!({"type": "message_stop"})),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::with_catalog_auth(
        http_client,
        "claude-opus-streaming",
        "claude-opus-streaming",
        ThinkingLevel::Disabled,
        std::sync::Arc::new(StaticHeaderAuth::default()),
    );

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");
    let buffered = parse_response(
        "claude-opus-streaming",
        "claude-opus-streaming",
        ThinkingLevel::Disabled,
        buffered,
        None,
    )
    .expect("buffered fixture should parse");

    assert_eq!(streamed, buffered);
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

fn text_response_body() -> serde_json::Value {
    json!({
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "Done"}],
        "usage": {"input_tokens": 2, "output_tokens": 1}
    })
}
