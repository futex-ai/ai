//! MiniMax transport and HTTP error tests.

use std::{collections::BTreeMap, error::Error, sync::Arc, time::Duration};

use ai_interface::{Effort, ErrorKind, Model, ModelError};
use ai_models_core::test_support::{SseFixture, recording_streaming_client};
use async_trait::async_trait;
use json_http::{JsonHttpAuth, JsonHttpHeaders};
use serde_json::json;
use unimock::Unimock;

use crate::minimax::MINIMAX_M3;

use super::{
    MiniMaxModel, response,
    support::{simple_request, unused_http_client},
};

#[tokio::test]
async fn classifies_http_and_transport_failures() {
    let (http_client, _) = recording_streaming_client(vec![SseFixture::OpeningError(
        json_http::Error::HttpStatus {
            headers: Default::default(),
            status: 429,
            body: json!({"error": {"message": "slow down"}}),
        },
    )]);
    let http_error = MiniMaxModel::new(http_client, MINIMAX_M3, "key")
        .complete(&simple_request())
        .await
        .expect_err("HTTP rate limit should fail");
    assert!(matches!(http_error.core().kind, ErrorKind::RateLimited));

    let (http_client, _) = recording_streaming_client(vec![SseFixture::OpeningError(
        json_http::Error::transport("connection reset"),
    )]);
    let transport_error = MiniMaxModel::new(http_client, MINIMAX_M3, "key")
        .complete(&simple_request())
        .await
        .expect_err("transport error should fail");
    assert!(matches!(transport_error.core().kind, ErrorKind::Transient));

    for (status, code, kind) in [
        (400, 1039, ErrorKind::ContextLimitExceeded),
        (401, 1039, ErrorKind::RejectedKey),
        (429, 1039, ErrorKind::RateLimited),
        (503, 1039, ErrorKind::Transient),
        (400, 1004, ErrorKind::InvalidRequest),
        (400, 1002, ErrorKind::InvalidRequest),
    ] {
        let mut headers = JsonHttpHeaders::new();
        headers.insert("Trace-Id", "req-context".parse().unwrap());
        headers.insert("x-request-id", "secondary-id".parse().unwrap());
        headers.insert("Retry-After", "7".parse().unwrap());
        let (client, requests) = recording_streaming_client(vec![SseFixture::OpeningError(
            json_http::Error::HttpStatus {
                status,
                headers: Box::new(headers),
                body: json!({"base_resp":{"status_code":code,"status_msg":"too much context"}}),
            },
        )]);
        let error = MiniMaxModel::new(client, MINIMAX_M3, "key")
            .with_clock(Arc::new(Unimock::new(())))
            .complete(&simple_request())
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind, "HTTP {status}, code {code}");
        assert_eq!(error.core().provider, "minimax");
        assert_eq!(error.core().model_id, "MiniMax-M3");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-context")
        );
        assert_eq!(error.core().retry_after, Some(Duration::from_secs(7)));
        assert_eq!(
            error.core().message,
            format!("HTTP {status}: base_resp status_code {code}: too much context")
        );
        assert!(error.source().unwrap().is::<json_http::Error>());
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn classifies_auth_and_response_shape_failures() {
    let auth_error =
        MiniMaxModel::with_auth(unused_http_client(), MINIMAX_M3, Arc::new(FailingAuth))
            .complete(&simple_request())
            .await
            .expect_err("auth hook error should fail");
    assert!(matches!(auth_error.core().kind, ErrorKind::Transient));

    let malformed_error = response::parse_response(
        MINIMAX_M3.provider_model_id.as_ref(),
        MINIMAX_M3.provider_model_id.as_ref(),
        Effort::Medium,
        json!({"choices": "not-an-array"}),
        None,
    )
    .expect_err("malformed typed response should fail");
    assert!(
        matches!(malformed_error, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::InvalidOutput)
    );
}

struct FailingAuth;

#[async_trait]
impl JsonHttpAuth for FailingAuth {
    async fn apply_headers(
        &self,
        _headers: &mut BTreeMap<String, String>,
    ) -> json_http::Result<()> {
        Err(json_http::Error::auth("credential unavailable"))
    }
}
