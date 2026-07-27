//! Kimi HTTP dispatch and error classification tests.

use ai_interface::{Model, ModelError};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpResponse, StaticHeaderAuth};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{KIMI_K3, KimiModel};

use super::{
    client::request_error,
    test_support::{
        recording_http_client, simple_request, successful_response, transport_failure_http_client,
        unused_http_client,
    },
};

#[tokio::test]
async fn sends_bearer_auth_to_exact_moonshot_endpoint() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = KimiModel::new(http_client, "moonshot-secret");

    model
        .complete(&simple_request())
        .await
        .expect("Kimi response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(
        requests[0].url,
        "https://api.moonshot.ai/v1/chat/completions"
    );
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer moonshot-secret")
    );
    assert_eq!(requests[0].body.as_ref().expect("body")["model"], KIMI_K3);
}

#[tokio::test]
async fn classifies_rate_transient_and_ordinary_statuses() {
    let cases = [
        (429, ErrorKind::RateLimited),
        (408, ErrorKind::Transient),
        (409, ErrorKind::Transient),
        (425, ErrorKind::Transient),
        (500, ErrorKind::Transient),
        (503, ErrorKind::Transient),
        (400, ErrorKind::Provider),
        (403, ErrorKind::Provider),
    ];

    for (status, expected) in cases {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            status,
            body: json!({"error": {"message": "request rejected"}}),
        });
        let error = KimiModel::new(http_client, "moonshot-secret")
            .complete(&simple_request())
            .await
            .expect_err("HTTP failure should return an error");

        assert!(expected.matches(&error), "status {status}: {error}");
        assert!(!error.to_string().contains("moonshot-secret"));
    }
}

#[tokio::test]
async fn transport_failures_are_transient_and_do_not_expose_credentials() {
    let model = KimiModel::new(
        transport_failure_http_client("connection reset"),
        "moonshot-secret",
    );
    let error = model
        .complete(&simple_request())
        .await
        .expect_err("transport failure should return an error");

    assert!(matches!(error, ModelError::TransientProvider { .. }));
    assert!(!error.to_string().contains("moonshot-secret"));
}

#[tokio::test]
async fn auth_hook_failures_are_transient() {
    let auth: std::sync::Arc<dyn JsonHttpAuth> = std::sync::Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth("auth hook rejected"))),
    ));
    let model = KimiModel::with_auth(unused_http_client(), auth);
    let error = model
        .complete(&simple_request())
        .await
        .expect_err("auth failure should return an error");

    assert!(matches!(error, ModelError::TransientProvider { .. }));
}

#[test]
fn classifies_auth_transport_and_local_codec_failures() {
    let auth = request_error(json_http::Error::auth("auth hook rejected"), KIMI_K3);
    let transport = request_error(json_http::Error::transport("offline"), KIMI_K3);
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let serialization = request_error(json_http::Error::SerializeRequest { source }, KIMI_K3);
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let deserialization = request_error(
        json_http::Error::DeserializeResponse {
            body: Value::Null,
            source,
        },
        KIMI_K3,
    );

    assert!(matches!(auth, ModelError::TransientProvider { .. }));
    assert!(matches!(transport, ModelError::TransientProvider { .. }));
    assert!(matches!(serialization, ModelError::Internal { .. }));
    assert!(matches!(deserialization, ModelError::Internal { .. }));
}

#[test]
fn explicit_auth_constructor_does_not_require_ambient_credentials() {
    let _model = KimiModel::with_auth(
        unused_http_client(),
        std::sync::Arc::new(StaticHeaderAuth::default()),
    );
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
