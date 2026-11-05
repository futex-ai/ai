//! Public indexed model-event tests for Anthropic streams.

use ai_interface::{
    ConversationMessage, Model, ModelError, ModelRequest, ModelResponseFold, StructuredOutputSchema,
};
use serde_json::{Value, json};

use crate::CLAUDE_SONNET_4_6;
use crate::anthropic::stream_support::{
    event, events_from_buffered_body, recording_streaming_client,
};

use super::AnthropicModel;

#[tokio::test]
async fn emits_ordered_reasoning_and_assistant_deltas_with_terminal_parity() {
    let body = json!({
        "stop_reason": "end_turn",
        "content": [
            {"type": "thinking", "thinking": "private thought", "signature": "signed"},
            {"type": "text", "text": "First"},
            {"type": "text", "text": "Second"}
        ],
        "usage": {"input_tokens": 4, "output_tokens": 3}
    });
    let (http_client, _) = recording_streaming_client(vec![events_from_buffered_body(body)]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    let mut stream = model
        .stream(&simple_request())
        .await
        .expect("stream should open");
    let mut fold = ModelResponseFold::new();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await.expect("event should succeed") {
        events.push(serde_json::to_value(&event).expect("event should serialize"));
        fold.push(event).expect("event should fold");
    }
    let response = fold.finish().expect("complete stream should fold");

    assert_eq!(
        events,
        vec![
            json!({"ReasoningDelta": {"part_index": 0, "delta": "private thought"}}),
            state_delta(
                "ReasoningDelta",
                0,
                json!({
                    "type": "thinking", "thinking": "private thought", "signature": "signed"
                })
            ),
            json!({"TextDelta": {"part_index": 1, "delta": "First"}}),
            state_delta("TextDelta", 1, json!({"type": "text", "text": "First"})),
            json!({"TextDelta": {"part_index": 2, "delta": "\nSecond"}}),
            state_delta("TextDelta", 2, json!({"type": "text", "text": "Second"})),
            usage_event(),
            json!({"Finish": {
                "finish_reason": "Stop",
                "provider": "anthropic",
                "model_id": "claude-sonnet-4-6",
                "catalog_model_id": "anthropic/claude-sonnet-4-6",
                "effort": "disabled"
            }}),
        ]
    );
    assert_eq!(assistant_text(&events), response.text());
    assert!(
        stream
            .next()
            .await
            .expect("finished stream should stay closed")
            .is_none()
    );
}

#[tokio::test]
async fn structured_completion_emits_text_and_validated_finish() {
    let body = json!({
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "{\"summary\":\"Done\"}"}],
        "usage": {"input_tokens": 4, "output_tokens": 3}
    });
    let (http_client, _) = recording_streaming_client(vec![events_from_buffered_body(body)]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    let mut request = simple_request();
    request.response_schema = Some(summary_schema());
    let mut stream = model.stream(&request).await.expect("stream should open");
    let mut fold = ModelResponseFold::new();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await.expect("event should succeed") {
        events.push(serde_json::to_value(&event).expect("event should serialize"));
        fold.push(event).expect("event should fold");
    }
    let response = fold.finish().expect("structured completion should succeed");

    assert_eq!(response.structured_output, Some(json!({"summary": "Done"})));
    assert_eq!(
        events,
        vec![
            json!({"TextDelta": {"part_index": 0, "delta": "{\"summary\":\"Done\"}"}}),
            state_delta(
                "TextDelta",
                0,
                json!({"type": "text", "text": "{\"summary\":\"Done\"}"})
            ),
            usage_event(),
            json!({"Finish": {
                "finish_reason": "Stop", "provider": "anthropic", "model_id": "claude-sonnet-4-6",
                "catalog_model_id": "anthropic/claude-sonnet-4-6", "effort": "disabled",
                "structured_output": {"summary": "Done"}
            }}),
        ]
    );
}

fn state_delta(kind: &str, part_index: usize, block: Value) -> Value {
    let mut payload = json!({"blocks": [block], "scope": "part"});
    if kind == "TextDelta" {
        payload["mode"] = json!("normalized_text");
    }
    json!({kind: {"part_index": part_index, "delta": "", "provider_state": {
        "provider": "anthropic", "model": "claude-sonnet-4-6", "version": 1,
        "payload": payload
    }}})
}

#[tokio::test]
async fn emits_partial_text_before_an_interruption() {
    let events = vec![
        event(
            "message_start",
            json!({
                "type": "message_start", "message": {"usage": {"input_tokens": 1, "output_tokens": 0}}
            }),
        ),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start", "index": 0,
                "content_block": {"type": "text", "text": ""}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta", "index": 0,
                "delta": {"type": "text_delta", "text": "partial"}
            }),
        ),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    let mut stream = model
        .stream(&simple_request())
        .await
        .expect("stream should open");
    let first = stream.next().await.expect("first text should arrive");
    let error = stream
        .next()
        .await
        .expect_err("interrupted completion should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(
        serde_json::to_value(first).expect("event should serialize"),
        json!({"TextDelta": {"part_index": 0, "delta": "partial"}})
    );
    assert!(
        stream
            .next()
            .await
            .expect("failed stream should stay closed")
            .is_none()
    );
}

fn usage_event() -> Value {
    json!({"Usage": {"usage": {
        "input_tokens": 4, "output_tokens": 3, "cached_input_tokens": 0,
        "reasoning_tokens": 0, "total_tokens": 7, "estimated_cost_microusd": 0
    }}})
}

fn assistant_text(events: &[Value]) -> String {
    events
        .iter()
        .filter_map(|event| event.get("TextDelta")?.get("delta")?.as_str())
        .collect()
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

fn summary_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object", "properties": {"summary": {"type": "string"}}, "required": ["summary"]
        }),
    }
}
