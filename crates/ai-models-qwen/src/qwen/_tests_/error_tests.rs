//! Qwen HTTP, transport, auth, and semantic error tests.

use std::sync::Arc;

use ai_interface::{Model, ModelError};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{QWEN_3_7_PLUS, QwenModel};

use super::{
    client::{classify_qwen_http_error, request_error},
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
        let error = QwenModel::new(http_client, "qwen-secret")
            .complete(&simple_request())
            .await
            .expect_err("HTTP failure should return an error");

        assert!(expected.matches(&error), "status {status}: {error}");
        assert!(!error.to_string().contains("qwen-secret"));
        assert!(!format!("{error:?}").contains("qwen-secret"));
    }
}

#[test]
fn recognizes_documented_input_length_message_in_both_error_shapes() {
    for body in [
        json!({
            "error": {
                "code": "InvalidParameter",
                "message": "Range of input length should be [1, 1000000]"
            }
        }),
        json!({
            "code": "InvalidParameter",
            "message": "Range of input length should be [1, 1000000]"
        }),
    ] {
        assert!(matches!(
            classify_qwen_http_error(QWEN_3_7_PLUS, 400, &body),
            ModelError::ContextLimitExceeded { .. }
        ));
    }

    let unrelated = classify_qwen_http_error(
        QWEN_3_7_PLUS,
        400,
        &json!({
            "error": {"code": "InvalidParameter", "message": "Invalid tool schema"}
        }),
    );
    assert!(matches!(unrelated, ModelError::Provider { .. }));
}

#[tokio::test]
async fn transport_and_auth_failures_are_transient() {
    let transport = QwenModel::new(
        transport_failure_http_client("connection reset"),
        "qwen-secret",
    )
    .complete(&simple_request())
    .await
    .expect_err("transport failure should fail");
    assert!(matches!(transport, ModelError::TransientProvider { .. }));
    assert!(!transport.to_string().contains("qwen-secret"));

    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth("credential unavailable"))),
    ));
    let auth_error = QwenModel::with_auth(unused_http_client(), auth)
        .complete(&simple_request())
        .await
        .expect_err("auth failure should fail");
    assert!(matches!(auth_error, ModelError::TransientProvider { .. }));
}

#[test]
fn serialization_and_deserialization_failures_are_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let serialization = request_error(json_http::Error::SerializeRequest { source }, QWEN_3_7_PLUS);
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let deserialization = request_error(
        json_http::Error::DeserializeResponse {
            body: Value::Null,
            source,
        },
        QWEN_3_7_PLUS,
    );

    assert!(matches!(serialization, ModelError::Internal { .. }));
    assert!(matches!(deserialization, ModelError::Internal { .. }));
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
