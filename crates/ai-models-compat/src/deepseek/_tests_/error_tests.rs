//! DeepSeek HTTP, transport, auth, and semantic error tests.

use ai_interface::ErrorKind;

use std::sync::Arc;

use ai_interface::{Model, ModelError};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::deepseek::{DEEPSEEK_V4_PRO, DeepSeekModel};

use super::{
    client::request_error,
    test_support::{
        recording_http_client, simple_request, transport_failure_http_client, unused_http_client,
    },
};

#[tokio::test]
async fn classifies_the_complete_http_status_matrix() {
    let cases = [
        (400, ErrorKind::InvalidRequest),
        (401, ErrorKind::RejectedKey),
        (402, ErrorKind::RejectedKey),
        (403, ErrorKind::RejectedKey),
        (404, ErrorKind::UnknownModel),
        (413, ErrorKind::InvalidRequest),
        (451, ErrorKind::InvalidRequest),
        (408, ErrorKind::Transient),
        (409, ErrorKind::Transient),
        (422, ErrorKind::InvalidRequest),
        (425, ErrorKind::Transient),
        (429, ErrorKind::RateLimited),
        (500, ErrorKind::Transient),
        (503, ErrorKind::Transient),
    ];

    for (status, expected) in cases {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            headers: Default::default(),
            status,
            body: json!({"error": {"message": "request rejected"}}),
        });
        let error = DeepSeekModel::new(http_client, "deepseek-secret")
            .complete(&simple_request())
            .await
            .expect_err("HTTP failure should return an error");

        assert!(expected == error.core().kind, "status {status}: {error}");
        assert!(!error.to_string().contains("deepseek-secret"));
        assert!(!format!("{error:?}").contains("deepseek-secret"));
    }
}

#[tokio::test]
async fn recognizes_structured_context_overflow_codes() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
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

    assert!(matches!(error.core().kind, ErrorKind::ContextLimitExceeded));
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
    assert!(matches!(transport.core().kind, ErrorKind::Transient));
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
    assert!(matches!(auth_error.core().kind, ErrorKind::Transient));
}

#[test]
fn request_serialization_and_response_deserialization_errors_are_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let serialization = request_error(
        json_http::Error::SerializeRequest { source },
        DEEPSEEK_V4_PRO.provider_model_id.as_ref(),
    );
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let deserialization = request_error(
        json_http::Error::DeserializeResponse {
            body: Value::Null,
            source,
        },
        DEEPSEEK_V4_PRO.provider_model_id.as_ref(),
    );

    assert!(
        matches!(serialization, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::Internal)
    );
    assert!(
        matches!(deserialization, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::InvalidOutput)
    );
}

#[tokio::test]
async fn completed_stream_without_choices_is_an_interruption() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({"choices": []}),
    });
    let error = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect_err("missing choices should fail");

    assert!(matches!(error.core().kind, ErrorKind::Interrupted));
}
