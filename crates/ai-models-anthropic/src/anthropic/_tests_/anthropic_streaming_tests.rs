//! Anthropic streaming request and buffered-response parity tests.

use std::time::Duration;

use ai_interface::{
    ConversationMessage, ConversationPart, Effort, FinishReason, Model, ModelRequest,
    ModelResponse, ModelUsage, ToolCall,
};
use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::CLAUDE_SONNET_4_6;
use crate::anthropic::stream_support::{
    client_for_buffered_bodies, event, expected_state, recording_streaming_client,
};

use super::{AnthropicModel, stream_support::custom_spec};

#[tokio::test]
async fn sends_streaming_request_with_completion_deadlines() {
    let body = text_response_body();
    let (http_client, requests) = client_for_buffered_bodies(vec![body]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");

    model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");

    let requests = requests
        .lock()
        .expect("request lock should not be poisoned");
    let request = &requests[0];
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            request
                .body
                .as_ref()
                .expect("request body")
                .as_json_bytes()
                .expect("JSON request body")
        )
        .expect("valid JSON request body")["stream"],
        true
    );
    assert_eq!(request.timeout, Duration::from_secs(3_600));
    assert_eq!(request.idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn explicit_total_timeout_overrides_streaming_default() {
    let (http_client, requests) = client_for_buffered_bodies(vec![text_response_body()]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
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
async fn streamed_deltas_preserve_complete_response_fields() {
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
        custom_spec("claude-opus-streaming"),
        std::sync::Arc::new(StaticHeaderAuth::default()),
    );

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");
    let buffered = ModelResponse {
        provider: "anthropic".to_owned(),
        model_id: "claude-opus-streaming".to_owned(),
        catalog_model_id: Some("anthropic/claude-opus-streaming".to_owned()),
        thinking_level: Some(Effort::Disabled),
        parts: vec![
            ConversationPart::reasoning("private thought").with_provider_state(expected_state(
                "claude-opus-streaming",
                json!({"type": "thinking", "thinking": "private thought", "signature": "signed"}),
            )),
            ConversationPart::text("Done").with_provider_state(expected_state(
                "claude-opus-streaming", json!({"type": "text", "text": "Done"}),
            )),
            ConversationPart::from_tool_call(ToolCall {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "root"}),
            operation_id: None,
            }).with_provider_state(expected_state("claude-opus-streaming", json!({
                "type": "tool_use", "id": "call_1", "name": "memory_read", "input": {"path": "root"}
            }))),
        ],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: ModelUsage {
            input_tokens: 130,
            output_tokens: 32,
            cached_input_tokens: 40,
            reasoning_tokens: 0,
            total_tokens: 202,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        },
        attempts: Vec::new(),
    };

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
