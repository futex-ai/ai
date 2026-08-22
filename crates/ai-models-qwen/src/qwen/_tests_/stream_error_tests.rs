//! Qwen stream failure classification tests.

use ai_interface::{Model, ModelError};
use ai_models_core::test_support::{SseFixture, data_event, event, recording_streaming_client};
use serde_json::json;

use crate::QwenModel;

use super::test_support::simple_request;

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
    let model = QwenModel::new(http_client, "key");

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
    let model = QwenModel::new(http_client, "key");

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
