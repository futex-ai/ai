//! Public completion-event tests for Kimi streams.

use ai_interface::{ModelError, StructuredOutputSchema};
use ai_models_core::test_support::{SseFixture, done_event, event, recording_streaming_client};
use serde_json::{Value, json};

use crate::stream_test_support::{
    assistant_delta, assistant_text, content_events, observe, reasoning_delta,
};

use super::{KimiModel, test_support::simple_request};

#[tokio::test]
async fn emits_reasoning_and_assistant_deltas_in_order_with_parity() {
    let events = completed_stream(vec![
        text_chunk(Some("Think"), None, None),
        text_chunk(None, Some("Hel"), None),
        text_chunk(Some("ing"), Some("lo"), Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = KimiModel::new(http_client, "key");
    let mut recorded = Vec::new();

    let response = observe(&model, &simple_request(), &mut recorded)
        .await
        .expect("event-observing completion should succeed");
    let events = content_events(&recorded);

    assert_eq!(
        events,
        vec![
            reasoning_delta("Think"),
            assistant_delta("Hel"),
            reasoning_delta("ing"),
            assistant_delta("lo"),
        ]
    );
    assert_eq!(assistant_text(&events), response.text());
}

#[tokio::test]
async fn structured_completion_emits_text_deltas() {
    let events = completed_stream(vec![
        text_chunk(None, Some("{\"summary\":"), None),
        text_chunk(None, Some("\"Done\"}"), Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = KimiModel::new(http_client, "key");
    let mut recorded = Vec::new();
    let mut request = simple_request();
    request.response_schema = Some(summary_schema());

    let response = observe(&model, &request, &mut recorded)
        .await
        .expect("structured completion should emit validated output");

    assert_eq!(response.structured_output, Some(json!({"summary": "Done"})));
    assert_eq!(assistant_text(&recorded), "{\"summary\":\"Done\"}");
}

#[tokio::test]
async fn emits_partial_text_before_an_interruption() {
    let events = vec![
        event(text_chunk(None, Some("partial"), None)),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = KimiModel::new(http_client, "key");
    let mut recorded = Vec::new();

    let error = observe(&model, &simple_request(), &mut recorded)
        .await
        .expect_err("interrupted completion should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(content_events(&recorded), vec![assistant_delta("partial")]);
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
