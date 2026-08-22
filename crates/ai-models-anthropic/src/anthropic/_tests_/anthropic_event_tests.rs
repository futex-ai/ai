//! Public completion-event tests for Anthropic streams.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, Model, ModelCompletionEvent, ModelCompletionEventSinkMock, ModelError,
    ModelRequest, StructuredOutputSchema,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::AnthropicModel;
use crate::anthropic::stream_support::{
    event, events_from_buffered_body, recording_streaming_client,
};

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
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let response = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .expect("event-observing completion should succeed");
    let events = recorded_events(&events);

    assert_eq!(
        events,
        vec![
            reasoning_delta("private thought"),
            assistant_delta("First"),
            assistant_delta("\nSecond"),
        ]
    );
    assert_eq!(assistant_text(&events), response.assistant_message);
}

#[tokio::test]
async fn structured_completion_suppresses_deltas() {
    let body = json!({
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "{\"summary\":\"Done\"}"}],
        "usage": {"input_tokens": 4, "output_tokens": 3}
    });
    let (http_client, _) = recording_streaming_client(vec![events_from_buffered_body(body)]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");
    let sink = Unimock::new(());
    let mut request = simple_request();
    request.response_schema = Some(summary_schema());

    let response = model
        .complete_with_events(&request, &sink)
        .await
        .expect("structured completion should succeed silently");

    assert_eq!(response.structured_output, Some(json!({"summary": "Done"})));
}

#[tokio::test]
async fn emits_partial_text_before_an_interruption() {
    let events = vec![
        event(
            "message_start",
            json!({
                "type": "message_start",
                "message": {"usage": {"input_tokens": 1, "output_tokens": 0}}
            }),
        ),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "text", "text": ""}
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "partial"}
            }),
        ),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(recorded.clone());

    let error = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .expect_err("interrupted completion should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(recorded_events(&recorded), vec![assistant_delta("partial")]);
}

fn recording_sink(events: Arc<Mutex<Vec<ModelCompletionEvent>>>) -> Unimock {
    Unimock::new(
        ModelCompletionEventSinkMock::emit
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, event| {
                events
                    .lock()
                    .expect("event lock should not be poisoned")
                    .push(event);
            })),
    )
}

fn recorded_events(events: &Mutex<Vec<ModelCompletionEvent>>) -> Vec<ModelCompletionEvent> {
    events
        .lock()
        .expect("event lock should not be poisoned")
        .clone()
}

fn assistant_text(events: &[ModelCompletionEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            ModelCompletionEvent::AssistantTextDelta { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect()
}

fn assistant_delta(delta: &str) -> ModelCompletionEvent {
    ModelCompletionEvent::AssistantTextDelta {
        delta: delta.to_owned(),
    }
}

fn reasoning_delta(delta: &str) -> ModelCompletionEvent {
    ModelCompletionEvent::ReasoningTextDelta {
        delta: delta.to_owned(),
    }
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
            "type": "object",
            "properties": {"summary": {"type": "string"}},
            "required": ["summary"]
        }),
    }
}
