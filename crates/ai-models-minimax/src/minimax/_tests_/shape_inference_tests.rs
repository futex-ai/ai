//! MiniMax M2.x stream-shape inference regression tests.

use ai_interface::{Model, ModelCompletionEvent, ModelResponse};
use ai_models_core::{
    ThinkingLevel,
    test_support::{
        SseFixture, StreamItem, done_event, event, recorded_completion_events,
        recording_completion_event_sink, recording_streaming_client,
    },
};
use serde_json::{Value, json};

use crate::{MINIMAX_M2_7, MiniMaxModel};

use super::{response, support::simple_request};

#[tokio::test]
async fn incremental_fragments_survive_empty_finish_and_match_buffered() {
    let events = completed_stream(vec![
        content_chunk("LIVE_", None),
        content_chunk("MODEL_", None),
        content_chunk("API_OK", None),
        content_chunk("", Some("stop")),
    ]);

    let (response, events) = complete_with_events(events).await;
    let buffered = buffered_response("LIVE_MODEL_API_OK", None);

    assert_eq!(response, buffered);
    assert_eq!(events, vec![assistant_delta("LIVE_MODEL_API_OK")],);
}

#[tokio::test]
async fn extending_snapshots_restore_terminal_text_once() {
    let events = completed_stream(vec![
        content_chunk("LIVE_", None),
        content_chunk("LIVE_MODEL_", None),
        content_chunk("LIVE_MODEL_API_OK", Some("stop")),
    ]);

    let (response, events) = complete_with_events(events).await;

    assert_eq!(response.assistant_message, "LIVE_MODEL_API_OK");
    assert_eq!(events, vec![assistant_delta("LIVE_MODEL_API_OK")],);
}

#[tokio::test]
async fn snapshot_evidence_allows_replacement() {
    let events = completed_stream(vec![
        content_chunk("Hel", None),
        content_chunk("Hello", None),
        content_chunk("Goodbye", Some("stop")),
    ]);

    let (response, events) = complete_with_events(events).await;

    assert_eq!(response.assistant_message, "Goodbye");
    assert_eq!(events, vec![assistant_delta("Goodbye")]);
}

#[tokio::test]
async fn empty_and_absent_terminal_content_preserve_fragments() {
    for terminal_delta in [json!({"content": ""}), json!({})] {
        let events = completed_stream(vec![
            content_chunk("LIVE_", None),
            content_chunk("MODEL_API_OK", None),
            json!({
                "choices": [{
                    "index": 0,
                    "delta": terminal_delta,
                    "finish_reason": "stop"
                }]
            }),
        ]);

        let (response, events) = complete_with_events(events).await;

        assert_eq!(response.assistant_message, "LIVE_MODEL_API_OK");
        assert_eq!(events, vec![assistant_delta("LIVE_MODEL_API_OK")],);
    }
}

#[tokio::test]
async fn reasoning_snapshots_remain_canonical_with_inferred_fragments() {
    let draft = reasoning_details("draft reasoning");
    let canonical = reasoning_details("canonical reasoning");
    let events = completed_stream(vec![
        content_and_reasoning_chunk("LIVE_", draft, None),
        content_and_reasoning_chunk("MODEL_", canonical.clone(), None),
        content_chunk("API_OK", Some("stop")),
    ]);

    let (response, events) = complete_with_events(events).await;
    let buffered = buffered_response("LIVE_MODEL_API_OK", Some(canonical));

    assert_eq!(response, buffered);
    assert_eq!(events, vec![assistant_delta("LIVE_MODEL_API_OK")],);
}

async fn complete_with_events(
    events: Vec<StreamItem>,
) -> (ModelResponse, Vec<ModelCompletionEvent>) {
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M2_7, "key");
    let (sink, recorded) = recording_completion_event_sink();
    let response = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .expect("M2.7 stream should complete");
    (response, recorded_completion_events(&recorded))
}

fn completed_stream(chunks: Vec<Value>) -> Vec<StreamItem> {
    let mut events = chunks.into_iter().map(event).collect::<Vec<_>>();
    events.push(event(json!({
        "choices": [],
        "usage": {"prompt_tokens": 4, "completion_tokens": 4, "total_tokens": 8}
    })));
    events.push(done_event());
    events
}

fn content_chunk(content: &str, finish_reason: Option<&str>) -> Value {
    json!({
        "choices": [{
            "index": 0,
            "delta": {"content": content},
            "finish_reason": finish_reason
        }]
    })
}

fn content_and_reasoning_chunk(
    content: &str,
    reasoning_details: Value,
    finish_reason: Option<&str>,
) -> Value {
    json!({
        "choices": [{
            "index": 0,
            "delta": {
                "content": content,
                "reasoning_details": reasoning_details
            },
            "finish_reason": finish_reason
        }]
    })
}

fn reasoning_details(text: &str) -> Value {
    json!([{
        "type": "reasoning.text",
        "id": "reasoning-1",
        "format": "MiniMax-response-v1",
        "index": 0,
        "text": text
    }])
}

fn buffered_response(content: &str, reasoning_details: Option<Value>) -> ModelResponse {
    let mut message = json!({"content": content});
    if let Some(reasoning_details) = reasoning_details {
        message["reasoning_details"] = reasoning_details;
    }
    response::parse_response(
        MINIMAX_M2_7,
        MINIMAX_M2_7,
        ThinkingLevel::Medium,
        json!({
            "choices": [{"message": message, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 4, "total_tokens": 8}
        }),
        None,
    )
    .expect("buffered MiniMax response should parse")
}

fn assistant_delta(delta: &str) -> ModelCompletionEvent {
    ModelCompletionEvent::AssistantTextDelta {
        delta: delta.to_owned(),
    }
}

#[tokio::test]
async fn repeated_identical_fragments_append_instead_of_collapsing() {
    let events = completed_stream(vec![
        content_chunk("ha", None),
        content_chunk("ha", None),
        content_chunk("!", Some("stop")),
    ]);

    let (response, events) = complete_with_events(events).await;

    assert_eq!(response.assistant_message, "haha!");
    assert_eq!(events, vec![assistant_delta("haha!")]);
}

#[tokio::test]
async fn a_value_extending_the_retained_prefix_is_a_snapshot_even_without_prior_evidence() {
    let events = completed_stream(vec![
        content_chunk("LIVE_", None),
        content_chunk("LIVE_MODEL_API_OK", Some("stop")),
    ]);

    let (response, _) = complete_with_events(events).await;

    assert_eq!(response.assistant_message, "LIVE_MODEL_API_OK");
}

#[tokio::test]
async fn a_repeated_snapshot_after_evidence_does_not_duplicate_text() {
    let events = completed_stream(vec![
        content_chunk("LIVE_", None),
        content_chunk("LIVE_MODEL_API_OK", None),
        content_chunk("LIVE_MODEL_API_OK", Some("stop")),
    ]);

    let (response, events) = complete_with_events(events).await;

    assert_eq!(response.assistant_message, "LIVE_MODEL_API_OK");
    assert_eq!(events, vec![assistant_delta("LIVE_MODEL_API_OK")]);
}
