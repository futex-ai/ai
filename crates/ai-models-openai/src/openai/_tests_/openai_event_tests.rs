//! Public completion-event tests for OpenAI Responses streams.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, Model, ModelCompletionEvent, ModelCompletionEventSinkMock, ModelError,
    ModelRequest, StructuredOutputSchema,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::OpenAiModel;
use crate::openai::stream_support::{event, recording_streaming_client, terminal_event};

#[tokio::test]
async fn emits_output_and_reasoning_deltas_in_provider_order_with_parity() {
    let events = vec![
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "Hel"}),
        ),
        event(
            "response.reasoning_summary_text.delta",
            json!({
                "type": "response.reasoning_summary_text.delta",
                "delta": "think"
            }),
        ),
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "lo"}),
        ),
        event(
            "response.reasoning_summary_text.delta",
            json!({
                "type": "response.reasoning_summary_text.delta",
                "delta": "ing"
            }),
        ),
        terminal_event("response.completed", text_body("Hello")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");
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
            assistant_delta("Hel"),
            reasoning_delta("think"),
            assistant_delta("lo"),
            reasoning_delta("ing"),
        ]
    );
    assert_eq!(assistant_text(&events), response.assistant_message);
}

#[tokio::test]
async fn structured_completion_suppresses_output_deltas() {
    let events = vec![
        event(
            "response.output_text.delta",
            json!({
                "type": "response.output_text.delta",
                "delta": "{\"summary\":\"Done\"}"
            }),
        ),
        terminal_event("response.completed", text_body("{\"summary\":\"Done\"}")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");
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
async fn emits_partial_output_before_an_interruption() {
    let events = vec![
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "partial"}),
        ),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");
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

fn text_body(text: &str) -> Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": text}]
        }],
        "usage": {"input_tokens": 4, "output_tokens": 2, "total_tokens": 6}
    })
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
