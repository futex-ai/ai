//! OpenAI Responses stream failure classification tests.

use ai_interface::{ConversationMessage, Model, ModelError, ModelRequest};
use serde_json::json;

use super::OpenAiModel;
use crate::openai::stream_support::{event, recording_streaming_client, terminal_event};

#[tokio::test]
async fn classifies_first_failed_and_error_events() {
    let failed = terminal_event(
        "response.failed",
        json!({
            "status": "failed",
            "error": {"code": "server_error", "message": "Generation failed"}
        }),
    );
    let rate_limit = event(
        "error",
        json!({
            "type": "error",
            "code": "rate_limit_exceeded",
            "message": "Slow down",
            "param": null
        }),
    );
    let (http_client, _) = recording_streaming_client(vec![vec![failed], vec![rate_limit]]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

    let failed = model
        .complete(&simple_request())
        .await
        .expect_err("failed response should fail");
    let rate_limit = model
        .complete(&simple_request())
        .await
        .expect_err("error event should fail");

    assert!(matches!(failed, ModelError::TransientProvider { .. }));
    assert!(matches!(rate_limit, ModelError::RateLimited { .. }));
}

#[tokio::test]
async fn terminal_failures_after_progress_are_interruptions() {
    let failed = terminal_event(
        "response.failed",
        json!({
            "status": "failed",
            "error": {"code": "server_error", "message": "Generation failed"}
        }),
    );
    let native_error = event(
        "error",
        json!({
            "type": "error",
            "code": "server_error",
            "message": "Generation failed",
            "param": null
        }),
    );
    let (http_client, _) = recording_streaming_client(vec![
        vec![progress_event(), failed],
        vec![progress_event(), native_error],
    ]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

    for label in ["failed terminal", "native error"] {
        let error = model
            .complete(&simple_request())
            .await
            .expect_err("partially generated stream should fail");
        assert!(matches!(error, ModelError::Interrupted { .. }), "{label}");
    }
}

#[tokio::test]
async fn classifies_eof_before_and_after_progress() {
    let (http_client, _) = recording_streaming_client(vec![vec![], vec![progress_event()]]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("empty stream should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("partial stream should fail");

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn classifies_transport_failure_before_and_after_progress() {
    let (http_client, _) = recording_streaming_client(vec![
        vec![Err(json_http::Error::transport("offline"))],
        vec![progress_event(), Err(json_http::Error::transport("reset"))],
    ]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

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
    let (http_client, _) = recording_streaming_client(vec![
        vec![malformed_event()],
        vec![progress_event(), malformed_event()],
    ]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

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

fn progress_event() -> crate::openai::stream_support::StreamItem {
    event(
        "response.created",
        json!({"type": "response.created", "response": {"status": "in_progress"}}),
    )
}

fn malformed_event() -> crate::openai::stream_support::StreamItem {
    Ok(Some(json_http::JsonHttpSseEvent {
        event: None,
        data: "not JSON".to_owned(),
    }))
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
