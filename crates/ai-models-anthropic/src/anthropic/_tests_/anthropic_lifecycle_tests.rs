//! Tool lifecycle and terminal validation through the public stream.

use ai_interface::{ConversationMessage, Model, ModelError, ModelRequest, StructuredOutputSchema};
use serde_json::json;

use crate::CLAUDE_SONNET_4_6;
use crate::anthropic::stream_support::{
    event, events_from_buffered_body, recording_streaming_client,
};

use super::AnthropicModel;

#[tokio::test]
async fn emits_indexed_tool_lifecycle_before_usage_and_finish() {
    let mut events = events_from_buffered_body(json!({
        "stop_reason": "tool_use",
        "content": [
            {"type": "text", "text": "  "},
            {"type": "tool_use", "id": "call_1", "name": "lookup", "input": {"key": "root"}}
        ],
        "usage": {"input_tokens": 2, "output_tokens": 1}
    }));
    events.splice(
        5..6,
        [
            event(
                "content_block_delta",
                json!({
                    "type": "content_block_delta", "index": 1,
                    "delta": {"type": "input_json_delta", "partial_json": "{\"key\":"}
                }),
            ),
            event(
                "content_block_delta",
                json!({
                    "type": "content_block_delta", "index": 1,
                    "delta": {"type": "input_json_delta", "partial_json": "\"root\"}"}
                }),
            ),
        ],
    );
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    let mut stream = model
        .stream(&simple_request())
        .await
        .expect("stream should open");
    let mut actual = Vec::new();
    while let Some(event) = stream.next().await.expect("event should succeed") {
        actual.push(serde_json::to_value(event).expect("event should serialize"));
    }

    assert_eq!(
        actual,
        vec![
            json!({"ToolCallStart": {"part_index": 0, "id": "call_1", "name": "lookup"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 0, "delta": "{\"key\":"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 0, "delta": "\"root\"}"}}),
            json!({"ToolCallEnd": {"part_index": 0}}),
            json!({"Usage": {"usage": {
                "input_tokens": 2, "output_tokens": 1, "cached_input_tokens": 0,
                "reasoning_tokens": 0, "total_tokens": 3, "estimated_cost_microusd": 0
            }}}),
            json!({"Finish": {
                "finish_reason": "ToolCalls", "provider": "anthropic", "model_id": "claude-sonnet-4-6",
                "catalog_model_id": "anthropic/claude-sonnet-4-6", "effort": "disabled"
            }}),
        ]
    );
}

#[tokio::test]
async fn emits_start_input_when_tool_arguments_have_no_fragments() {
    let events = vec![
        event(
            "message_start",
            json!({"type": "message_start", "message": {}}),
        ),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start", "index": 0,
                "content_block": {"type": "tool_use", "id": "call_2", "name": "status", "input": {"ready": true}}
            }),
        ),
        event(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 0}),
        ),
        event(
            "message_delta",
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}}),
        ),
        event("message_stop", json!({"type": "message_stop"})),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    let mut stream = model
        .stream(&simple_request())
        .await
        .expect("stream should open");
    let mut actual = Vec::new();
    while let Some(event) = stream.next().await.expect("event should succeed") {
        actual.push(serde_json::to_value(event).expect("event should serialize"));
    }

    assert_eq!(
        &actual[..actual.len().min(3)],
        &[
            json!({"ToolCallStart": {"part_index": 0, "id": "call_2", "name": "status"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 0, "delta": "{\"ready\":true}"}}),
            json!({"ToolCallEnd": {"part_index": 0}}),
        ]
    );
}

#[tokio::test]
async fn validates_terminal_output_before_usage_and_closes_after_failure() {
    let bodies = vec![
        json!({"stop_reason": "tool_use", "content": []}),
        json!({"stop_reason": "end_turn", "content": [{"type": "text", "text": "{}"}]}),
    ];
    let (http_client, _) =
        recording_streaming_client(bodies.into_iter().map(events_from_buffered_body).collect());
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    let mut request = simple_request();
    let mut stream = model.stream(&request).await.expect("stream should open");
    let missing_call = stream.next().await.expect_err("tool finish needs a call");
    assert!(matches!(missing_call, ModelError::Provider { .. }));
    assert!(
        stream
            .next()
            .await
            .expect("failed stream should close")
            .is_none()
    );

    request.response_schema = Some(StructuredOutputSchema {
        name: "summary".to_owned(),
        schema: json!({
            "type": "object", "properties": {"summary": {"type": "string"}}, "required": ["summary"]
        }),
    });
    let mut stream = model.stream(&request).await.expect("stream should open");
    let first = stream.next().await.expect("text should arrive");
    assert_eq!(
        serde_json::to_value(first).expect("event should serialize"),
        json!({"TextDelta": {"part_index": 0, "delta": "{}"}})
    );
    let invalid_schema = stream
        .next()
        .await
        .expect_err("schema should fail before usage");
    assert!(matches!(invalid_schema, ModelError::Provider { .. }));
    assert!(
        stream
            .next()
            .await
            .expect("failed stream should close")
            .is_none()
    );
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
