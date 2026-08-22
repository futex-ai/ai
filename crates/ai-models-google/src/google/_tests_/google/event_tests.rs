//! Public completion-event tests for Google generate-content streams.

use std::sync::{Arc, Mutex};

use ai_interface::{
    Model, ModelCompletionEvent, ModelCompletionEventSinkMock, ModelError, StructuredOutputSchema,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn emits_thought_and_assistant_fragments_with_part_boundary_parity() {
    let chunks = vec![
        json!({"candidates": [{"content": {"parts": [
            {"text": "hidden ", "thought": true}
        ]}}]}),
        json!({"candidates": [{"content": {"parts": [
            {"text": "reasoning", "thought": true}
        ]}}]}),
        json!({"candidates": [{"content": {"parts": [{"text": "First"}]}}]}),
        json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {"parts": [{"text": "Second"}, {"text": "Third"}]}
            }],
            "usageMetadata": {"promptTokenCount": 4, "candidatesTokenCount": 3}
        }),
    ];
    let stream = chunks.into_iter().map(stream_support::event).collect();
    let (http_client, _) = stream_support::recording_streaming_client(vec![stream]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(recorded.clone());

    let response = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .expect("event-observing completion should succeed");
    let events = recorded_events(&recorded);

    assert_eq!(
        events,
        vec![
            reasoning_delta("hidden "),
            reasoning_delta("reasoning"),
            assistant_delta("First"),
            assistant_delta("Second"),
            assistant_delta("\nThird"),
        ]
    );
    assert_eq!(assistant_text(&events), response.assistant_message);
}

#[tokio::test]
async fn structured_completion_suppresses_candidate_text() {
    let chunks = vec![
        json!({"candidates": [{"content": {"parts": [
            {"text": "{\"summary\":"}
        ]}}]}),
        json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {"parts": [{"text": "\"Done\"}"}]}
            }]
        }),
    ];
    let stream = chunks.into_iter().map(stream_support::event).collect();
    let (http_client, _) = stream_support::recording_streaming_client(vec![stream]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
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
async fn emits_partial_candidate_text_before_an_interruption() {
    let stream = vec![
        stream_support::event(
            json!({"candidates": [{"content": {"parts": [{"text": "partial"}]}}]}),
        ),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = stream_support::recording_streaming_client(vec![stream]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
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
