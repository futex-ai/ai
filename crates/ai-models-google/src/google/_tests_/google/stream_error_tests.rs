//! Google generate-content stream failure classification tests.

use ai_interface::{Model, ModelError};
use serde_json::json;

use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn first_provider_error_preserves_status_classification() {
    let events = vec![stream_support::event(json!({
        "error": {"code": 503, "message": "Unavailable", "status": "UNAVAILABLE"}
    }))];
    let (http_client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    let error = model
        .complete(&simple_request())
        .await
        .expect_err("provider error should fail");

    assert!(matches!(error, ModelError::TransientProvider { .. }));
}

#[tokio::test]
async fn provider_error_after_progress_is_an_interruption() {
    let events = vec![
        progress_event(),
        stream_support::event(json!({
            "error": {"code": 503, "message": "Unavailable", "status": "UNAVAILABLE"}
        })),
    ];
    let (http_client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    let error = model
        .complete(&simple_request())
        .await
        .expect_err("partially generated stream should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn classifies_eof_before_and_after_progress() {
    let (http_client, _) =
        stream_support::recording_streaming_client(vec![vec![], vec![progress_event()]]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("empty stream should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("unterminated stream should fail");

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn classifies_transport_failure_before_and_after_progress() {
    let (http_client, _) = stream_support::recording_streaming_client(vec![
        vec![Err(json_http::Error::transport("offline"))],
        vec![progress_event(), Err(json_http::Error::transport("reset"))],
    ]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("opening transport failure should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("mid-stream transport failure should fail");

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn malformed_events_follow_progress_classification() {
    let (http_client, _) = stream_support::recording_streaming_client(vec![
        vec![malformed_event()],
        vec![progress_event(), malformed_event()],
    ]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("malformed first event should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("malformed event after progress should fail");

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

fn progress_event() -> stream_support::StreamItem {
    stream_support::event(json!({
        "candidates": [{"content": {"parts": [{"text": "partial"}]}}]
    }))
}

fn malformed_event() -> stream_support::StreamItem {
    Ok(Some(json_http::JsonHttpSseEvent {
        event: None,
        data: "not JSON".to_owned(),
    }))
}
