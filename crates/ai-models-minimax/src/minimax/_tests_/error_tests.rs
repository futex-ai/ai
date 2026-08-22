//! MiniMax transport and HTTP error tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Model, ModelError};
use ai_models_core::{
    ThinkingLevel,
    test_support::{SseFixture, recording_streaming_client},
};
use async_trait::async_trait;
use json_http::JsonHttpAuth;
use serde_json::json;

use crate::MINIMAX_M3;

use super::{
    MiniMaxModel, response,
    support::{simple_request, unused_http_client},
};

#[tokio::test]
async fn classifies_http_and_transport_failures() {
    let (http_client, _) = recording_streaming_client(vec![SseFixture::OpeningError(
        json_http::Error::HttpStatus {
            status: 429,
            body: json!({"error": {"message": "slow down"}}),
        },
    )]);
    let http_error = MiniMaxModel::new(http_client, MINIMAX_M3, "key")
        .complete(&simple_request())
        .await
        .expect_err("HTTP rate limit should fail");
    assert!(matches!(http_error, ModelError::RateLimited { .. }));

    let (http_client, _) = recording_streaming_client(vec![SseFixture::OpeningError(
        json_http::Error::transport("connection reset"),
    )]);
    let transport_error = MiniMaxModel::new(http_client, MINIMAX_M3, "key")
        .complete(&simple_request())
        .await
        .expect_err("transport error should fail");
    assert!(matches!(
        transport_error,
        ModelError::TransientProvider { .. }
    ));
}

#[tokio::test]
async fn classifies_auth_and_response_shape_failures() {
    let auth_error =
        MiniMaxModel::with_auth(unused_http_client(), MINIMAX_M3, Arc::new(FailingAuth))
            .complete(&simple_request())
            .await
            .expect_err("auth hook error should fail");
    assert!(matches!(auth_error, ModelError::TransientProvider { .. }));

    let malformed_error = response::parse_response(
        MINIMAX_M3,
        MINIMAX_M3,
        ThinkingLevel::Medium,
        json!({"choices": "not-an-array"}),
        None,
    )
    .expect_err("malformed typed response should fail");
    assert!(matches!(malformed_error, ModelError::Internal { .. }));
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
