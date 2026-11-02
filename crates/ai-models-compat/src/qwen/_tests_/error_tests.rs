//! Qwen HTTP, transport, auth, and semantic error tests.

use ai_interface::ErrorKind;

use std::sync::Arc;

use ai_interface::{Model, ModelError};
use ai_models_core::{ErrorContext, SystemClock};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::qwen::{QWEN_3_7_PLUS, QwenModel};

use super::{
    client::request_error,
    dialect::classify_qwen_http_error,
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
        let error = QwenModel::new(http_client, "qwen-secret")
            .complete(&simple_request())
            .await
            .expect_err("HTTP failure should return an error");

        assert!(expected == error.core().kind, "status {status}: {error}");
        assert!(!error.to_string().contains("qwen-secret"));
        assert!(!format!("{error:?}").contains("qwen-secret"));
    }
}

#[test]
fn recognizes_documented_input_length_message_in_both_error_shapes() {
    let context = ErrorContext::new(
        "qwen",
        QWEN_3_7_PLUS.provider_model_id.as_ref(),
        &[],
        Arc::new(SystemClock),
    );
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
            classify_qwen_http_error(&context, 400, &Default::default(), &body)
                .core()
                .kind,
            ErrorKind::ContextLimitExceeded
        ));
    }

    let unrelated = classify_qwen_http_error(
        &context,
        400,
        &Default::default(),
        &json!({
            "error": {"code": "InvalidParameter", "message": "Invalid tool schema"}
        }),
    );
    assert!(matches!(unrelated.core().kind, ErrorKind::InvalidRequest));
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
    assert!(matches!(transport.core().kind, ErrorKind::Transient));
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
    assert!(matches!(auth_error.core().kind, ErrorKind::Transient));
}

#[test]
fn serialization_and_deserialization_failures_are_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let serialization = request_error(
        json_http::Error::SerializeRequest { source },
        QWEN_3_7_PLUS.provider_model_id.as_ref(),
    );
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let deserialization = request_error(
        json_http::Error::DeserializeResponse {
            body: Value::Null,
            source,
        },
        QWEN_3_7_PLUS.provider_model_id.as_ref(),
    );

    assert!(
        matches!(serialization, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::Internal)
    );
    assert!(
        matches!(deserialization, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::InvalidOutput)
    );
}
