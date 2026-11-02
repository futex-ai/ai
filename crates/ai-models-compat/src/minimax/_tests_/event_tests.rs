//! Public completion-event tests for MiniMax streams.

use ai_interface::{ModelError, StructuredOutputSchema};
use ai_models_core::test_support::{SseFixture, done_event, event, recording_streaming_client};
use serde_json::{Value, json};

use crate::stream_test_support::{
    assistant_delta, assistant_text, content_events, observe, reasoning_delta,
};

use crate::minimax::{MINIMAX_M2_7, MINIMAX_M3, MiniMaxModel};

use super::support::simple_request;

#[tokio::test]
async fn emits_terminal_cumulative_content_and_append_only_reasoning_with_parity() {
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
    let mut recorded = Vec::new();

    let response = observe(&model, &simple_request(), &mut recorded)
        .await
        .expect("event-observing completion should succeed");
    let events = content_events(&recorded);

    assert_eq!(
        events,
        vec![
            reasoning_delta("Think"),
            reasoning_delta("ing"),
            assistant_delta("Hello"),
        ]
    );
    assert_eq!(assistant_text(&events), response.assistant_message);
}

#[tokio::test]
async fn structured_completion_emits_normalized_content() {
    let events = completed_stream(vec![
        text_chunk("{\"summary\":", None, "", None),
        text_chunk("{\"summary\":\"Done\"}", None, "", Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M2_7, "key");
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
async fn emits_partial_normalized_content_before_an_interruption() {
    let events = vec![
        event(text_chunk("partial", None, "", None)),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M3, "key");
    let mut recorded = Vec::new();

    let error = observe(&model, &simple_request(), &mut recorded)
        .await
        .expect_err("interrupted completion should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(content_events(&recorded), vec![assistant_delta("partial")]);
}

#[tokio::test]
async fn revised_cumulative_content_emits_only_the_terminal_snapshot() {
    let events = completed_stream(vec![
        text_chunk("dra", None, "", None),
        text_chunk("draft", None, "", None),
        text_chunk("replacement", None, "", Some("stop")),
        usage_chunk(),
    ]);
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M2_7, "key");
    let mut recorded = Vec::new();

    let response = observe(&model, &simple_request(), &mut recorded)
        .await
        .expect("revised cumulative stream should succeed");
    let events = content_events(&recorded);

    assert_eq!(events, vec![assistant_delta("replacement")]);
    assert_eq!(assistant_text(&events), response.assistant_message);
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
