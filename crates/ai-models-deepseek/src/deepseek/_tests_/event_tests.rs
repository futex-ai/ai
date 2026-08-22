//! Public completion-event tests for DeepSeek streams.

use ai_interface::{Model, ModelCompletionEvent, ModelError, StructuredOutputSchema};
use ai_models_core::test_support::{
    SseFixture, done_event, event, recorded_completion_events, recording_completion_event_sink,
    recording_streaming_client,
};
use serde_json::{Value, json};
use unimock::Unimock;

use super::{DeepSeekModel, test_support::simple_request};

#[tokio::test]
async fn emits_reasoning_and_assistant_deltas_in_order_with_parity() {
    let events = completed_stream(vec![
        text_chunk(Some("Think"), None, None),
        text_chunk(None, Some("Hel"), None),
        text_chunk(Some("ing"), Some("lo"), Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = DeepSeekModel::new(http_client, "key");
    let (sink, recorded) = recording_completion_event_sink();

    let response = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .expect("event-observing completion should succeed");
    let events = recorded_completion_events(&recorded);

    assert_eq!(
        events,
        vec![
            reasoning_delta("Think"),
            assistant_delta("Hel"),
            reasoning_delta("ing"),
            assistant_delta("lo"),
        ]
    );
    assert_eq!(assistant_text(&events), response.assistant_message);
}

#[tokio::test]
async fn structured_completion_suppresses_text_deltas() {
    let events = completed_stream(vec![
        text_chunk(None, Some("{\"summary\":"), None),
        text_chunk(None, Some("\"Done\"}"), Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = DeepSeekModel::new(http_client, "key");
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
        event(text_chunk(None, Some("partial"), None)),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = DeepSeekModel::new(http_client, "key");
    let (sink, recorded) = recording_completion_event_sink();

    let error = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .expect_err("interrupted completion should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(
        recorded_completion_events(&recorded),
        vec![assistant_delta("partial")]
    );
}

fn completed_stream(chunks: Vec<Value>) -> Vec<ai_models_core::test_support::StreamItem> {
    let mut events = chunks.into_iter().map(event).collect::<Vec<_>>();
    events.push(done_event());
    events
}

fn text_chunk(
    reasoning: Option<&str>,
    content: Option<&str>,
    finish_reason: Option<&str>,
) -> Value {
    json!({
        "choices": [{
            "index": 0,
            "delta": {"reasoning_content": reasoning, "content": content},
            "finish_reason": finish_reason
        }]
    })
}

fn usage_chunk() -> Value {
    json!({
        "choices": [],
        "usage": {"prompt_tokens": 4, "completion_tokens": 4, "total_tokens": 8}
    })
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
