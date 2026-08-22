//! Public completion-event tests for MiniMax streams.

use ai_interface::{Model, ModelCompletionEvent, ModelError, StructuredOutputSchema};
use ai_models_core::test_support::{
    SseFixture, done_event, event, recorded_completion_events, recording_completion_event_sink,
    recording_streaming_client,
};
use serde_json::{Value, json};
use unimock::Unimock;

use crate::{MINIMAX_M2_7, MiniMaxModel};

use super::support::simple_request;

#[tokio::test]
async fn emits_normalized_content_and_append_only_reasoning_with_parity() {
    let events = completed_stream(vec![
        text_chunk("Hel", Some("Think"), "discarded snapshot", None),
        text_chunk(
            "Hello",
            Some("ing"),
            "canonical revised snapshot",
            Some("stop"),
        ),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M2_7, "key");
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
async fn structured_completion_suppresses_normalized_content() {
    let events = completed_stream(vec![
        text_chunk("{\"summary\":", None, "", None),
        text_chunk("{\"summary\":\"Done\"}", None, "", Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M2_7, "key");
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
async fn emits_partial_normalized_content_before_an_interruption() {
    let events = vec![
        event(text_chunk("partial", None, "", None)),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M2_7, "key");
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
    content: &str,
    reasoning_content: Option<&str>,
    reasoning_detail: &str,
    finish_reason: Option<&str>,
) -> Value {
    json!({
        "choices": [{
            "index": 0,
            "delta": {
                "content": content,
                "reasoning_content": reasoning_content,
                "reasoning_details": [{
                    "type": "reasoning.text",
                    "id": "reasoning-1",
                    "format": "MiniMax-response-v1",
                    "index": 0,
                    "text": reasoning_detail
                }]
            },
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
