//! DeepSeek HTTP, transport, auth, and semantic error tests.

use std::sync::Arc;

use ai_interface::{Model, ModelError};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{DEEPSEEK_V4_PRO, DeepSeekModel};

use super::{
    client::request_error,
    test_support::{
        recording_http_client, simple_request, transport_failure_http_client, unused_http_client,
    },
};

#[tokio::test]
async fn classifies_the_complete_http_status_matrix() {
    let cases = [
        (400, ErrorKind::Provider),
        (401, ErrorKind::Provider),
        (402, ErrorKind::Provider),
        (408, ErrorKind::Transient),
        (409, ErrorKind::Transient),
        (422, ErrorKind::Provider),
        (425, ErrorKind::Transient),
        (429, ErrorKind::RateLimited),
        (500, ErrorKind::Transient),
        (503, ErrorKind::Transient),
    ];

    for (status, expected) in cases {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            status,
            body: json!({"error": {"message": "request rejected"}}),
        });
        let error = DeepSeekModel::new(http_client, "deepseek-secret")
            .complete(&simple_request())
            .await
            .expect_err("HTTP failure should return an error");

        assert!(expected.matches(&error), "status {status}: {error}");
        assert!(!error.to_string().contains("deepseek-secret"));
        assert!(!format!("{error:?}").contains("deepseek-secret"));
    }
}

#[tokio::test]
async fn recognizes_structured_context_overflow_codes() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 400,
        body: json!({
            "error": {
                "code": "context_length_exceeded",
                "message": "request is too large"
            }
        }),
    });
    let error = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect_err("context overflow should fail");

    assert!(matches!(error, ModelError::ContextLimitExceeded { .. }));
}

#[tokio::test]
async fn transport_and_auth_failures_are_transient() {
    let transport = DeepSeekModel::new(
        transport_failure_http_client("connection reset"),
        "deepseek-secret",
    )
    .complete(&simple_request())
    .await
    .expect_err("transport failure should fail");
    assert!(matches!(transport, ModelError::TransientProvider { .. }));
    assert!(!transport.to_string().contains("deepseek-secret"));

    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth("credential unavailable"))),
    ));
    let auth_error = DeepSeekModel::with_auth(unused_http_client(), auth)
        .complete(&simple_request())
        .await
        .expect_err("auth failure should fail");
    assert!(matches!(auth_error, ModelError::TransientProvider { .. }));
}

#[test]
fn request_serialization_and_response_deserialization_errors_are_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let serialization = request_error(
        json_http::Error::SerializeRequest { source },
        DEEPSEEK_V4_PRO,
    );
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let deserialization = request_error(
        json_http::Error::DeserializeResponse {
            body: Value::Null,
            source,
        },
        DEEPSEEK_V4_PRO,
    );

    assert!(matches!(serialization, ModelError::Internal { .. }));
    assert!(matches!(deserialization, ModelError::Internal { .. }));
}

#[tokio::test]
async fn missing_choices_remains_a_semantic_provider_failure() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({"choices": []}),
    });
    let error = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect_err("missing choices should fail");

    assert!(matches!(error, ModelError::Provider { .. }));
}

#[derive(Clone, Copy)]
enum ErrorKind {
    RateLimited,
    Transient,
    Provider,
}

impl ErrorKind {
    fn matches(self, error: &ModelError) -> bool {
        matches!(
            (self, error),
            (Self::RateLimited, ModelError::RateLimited { .. })
                | (Self::Transient, ModelError::TransientProvider { .. })
                | (Self::Provider, ModelError::Provider { .. })
        )
    }
}
