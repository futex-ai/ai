//! xAI synchronous stream failure classification tests.

use ai_interface::{ConversationMessage, Model, ModelError, ModelRequest};
use ai_models_core::test_support::{SseFixture, data_event, event, recording_streaming_client};
use serde_json::json;

use super::XaiModel;

#[tokio::test]
async fn provider_errors_preserve_classification_until_progress() {
    let provider_error = || {
        event(json!({
            "error": {"code": "rate_limit_exceeded", "message": "slow down"}
        }))
    };
    let (http_client, _) = recording_streaming_client(vec![
        SseFixture::Stream(vec![provider_error()]),
        SseFixture::Stream(vec![progress_event(), provider_error()]),
    ]);
    let model = XaiModel::new(http_client, "grok-4.5", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("provider error should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("provider error after progress should fail");

    assert!(matches!(before, ModelError::RateLimited { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn eof_transport_and_malformed_events_are_progress_aware() {
    let (http_client, _) = recording_streaming_client(vec![
        SseFixture::Stream(vec![]),
        SseFixture::Stream(vec![progress_event()]),
        SseFixture::Stream(vec![Err(json_http::Error::transport("offline"))]),
        SseFixture::Stream(vec![
            progress_event(),
            Err(json_http::Error::transport("reset")),
        ]),
        SseFixture::Stream(vec![data_event("not JSON")]),
        SseFixture::Stream(vec![progress_event(), data_event("not JSON")]),
    ]);
    let model = XaiModel::new(http_client, "grok-4.5", "key");

    for (position, interrupted) in [false, true, false, true, false, true]
        .into_iter()
        .enumerate()
    {
        let error = model
            .complete(&simple_request())
            .await
            .expect_err("incomplete stream should fail");
        assert_eq!(
            matches!(error, ModelError::Interrupted { .. }),
            interrupted,
            "fixture {position}: {error}"
        );
        if !interrupted {
            assert!(matches!(error, ModelError::TransientProvider { .. }));
        }
    }
}

fn progress_event() -> ai_models_core::test_support::StreamItem {
    event(json!({
        "choices": [{
            "index": 0,
            "delta": {"content": "partial"},
            "finish_reason": null
        }]
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
