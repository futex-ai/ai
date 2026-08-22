//! MiniMax stream failure classification tests.

use ai_interface::{Model, ModelError};
use ai_models_core::test_support::{SseFixture, data_event, event, recording_streaming_client};
use serde_json::json;

use crate::{MINIMAX_M3, MiniMaxModel};

use super::support::simple_request;

#[tokio::test]
async fn numeric_provider_errors_preserve_classification_until_progress() {
    let provider_error = || {
        event(json!({
            "choices": [],
            "base_resp": {"status_code": 1002, "status_msg": "rate limited"}
        }))
    };
    let (http_client, _) = recording_streaming_client(vec![
        SseFixture::Stream(vec![provider_error()]),
        SseFixture::Stream(vec![progress_event(), provider_error()]),
    ]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M3, "key");

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
    let model = MiniMaxModel::new(http_client, MINIMAX_M3, "key");

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

#[tokio::test]
async fn rejects_replaced_cumulative_content_and_reasoning() {
    let (http_client, _) = recording_streaming_client(vec![
        SseFixture::Stream(vec![content_event("first"), content_event("replacement")]),
        SseFixture::Stream(vec![
            reasoning_event("private"),
            reasoning_event("replacement"),
        ]),
    ]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M3, "key");

    for position in 0..2 {
        let error = model
            .complete(&simple_request())
            .await
            .expect_err("replaced cumulative state should fail");
        assert!(
            matches!(error, ModelError::Interrupted { .. }),
            "fixture {position}: {error}"
        );
    }
}

fn progress_event() -> ai_models_core::test_support::StreamItem {
    content_event("partial")
}

fn content_event(content: &str) -> ai_models_core::test_support::StreamItem {
    event(json!({
        "choices": [{
            "index": 0,
            "delta": {"content": content},
            "finish_reason": null
        }]
    }))
}

fn reasoning_event(text: &str) -> ai_models_core::test_support::StreamItem {
    event(json!({
        "choices": [{
            "index": 0,
            "delta": {
                "reasoning_details": [{
                    "type": "reasoning.text",
                    "index": 0,
                    "text": text
                }]
            },
            "finish_reason": null
        }]
    }))
}
