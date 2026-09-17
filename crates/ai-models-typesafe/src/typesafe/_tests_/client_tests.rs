//! TypeSafe judgment HTTP dispatch and error classification tests.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ai_interface::{JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentRequest};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpBody, JsonHttpMethod, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::{
    TypeSafeJudgmentModel,
    test_support::{
        recording_http_client, simple_request, successful_response, transport_failure_http_client,
        unused_http_client,
    },
};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn sends_the_exact_bearer_authenticated_request() {
    let (http_client, requests) = recording_http_client(successful_response());
    let model = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY);

    let response = model
        .judge(&simple_request())
        .await
        .expect("valid response should parse");

    assert_eq!(response.provider, "typesafe");
    assert_eq!(response.model_id, "jev-latest");
    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
    assert_eq!(requests[0].url, "https://api.typesafe.ai/v1/systemone");
    assert_eq!(requests[0].timeout, Duration::from_secs(60));
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer typesafe-secret-key")
    );
    assert_eq!(
        requests[0]
            .body
            .as_ref()
            .and_then(JsonHttpBody::as_json)
            .expect("request body should be JSON"),
        &json!({
            "state": "A payout has been delayed for three days.",
            "model": "jev-latest",
            "questions": {
                "is_urgent": {
                    "type": "noul",
                    "instructions": "Is this urgent?"
                }
            }
        })
    );
}

#[tokio::test]
async fn classifies_rate_limited_and_transient_statuses() {
    for (status, expected) in [
        (429, ErrorKind::RateLimited),
        (529, ErrorKind::Transient),
        (503, ErrorKind::Transient),
    ] {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            status,
            body: json!({"detail": {"message": "provider overloaded"}}),
        });
        let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert!(expected.matches(&error), "status {status}: {error}");
        assert_provider_fields(&error, "provider overloaded");
        assert_no_api_key(&error);
    }
}

#[tokio::test]
async fn terminal_statuses_use_nested_detail_messages() {
    for status in [401, 403, 422] {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            status,
            body: json!({
                "detail": {
                    "error_type": "authentication_error",
                    "message": "request rejected"
                }
            }),
        });
        let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert!(matches!(error, JudgmentError::Provider { .. }));
        assert_provider_fields(&error, "request rejected");
        assert_no_api_key(&error);
    }
}

#[tokio::test]
async fn message_extraction_uses_detail_and_body_fallbacks() {
    for (body, expected) in [
        (json!({"detail": "plain rejection"}), "plain rejection"),
        (json!("text rejection"), "text rejection"),
        (json!({"code": "bad_request"}), "{\"code\":\"bad_request\"}"),
        (Value::Null, "HTTP 400"),
    ] {
        let (http_client, _) = recording_http_client(JsonHttpResponse { status: 400, body });
        let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert_provider_fields(&error, expected);
        assert_no_api_key(&error);
    }
}

#[tokio::test]
async fn transport_and_auth_hook_failures_are_transient() {
    let transport = TypeSafeJudgmentModel::new(
        transport_failure_http_client("connection reset"),
        "jev-latest",
        API_KEY,
    )
    .judge(&simple_request())
    .await
    .expect_err("transport failure should return an error");
    assert!(matches!(transport, JudgmentError::TransientProvider { .. }));
    assert_no_api_key(&transport);

    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth("credential unavailable"))),
    ));
    let auth_error = TypeSafeJudgmentModel::with_auth(unused_http_client(), "jev-latest", auth)
        .judge(&simple_request())
        .await
        .expect_err("auth-hook failure should return an error");
    assert!(matches!(
        auth_error,
        JudgmentError::TransientProvider { .. }
    ));
    assert_no_api_key(&auth_error);
}

#[tokio::test]
async fn malformed_success_body_is_a_terminal_provider_error() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({"model": "jev-1.13.0", "answers": "invalid"}),
    });
    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("malformed response should return an error");

    assert!(matches!(
        &error,
        JudgmentError::Provider { message, .. } if message == "malformed provider payload"
    ));
    assert_no_api_key(&error);
}

#[tokio::test]
async fn local_validation_returns_before_transport() {
    let model = TypeSafeJudgmentModel::new(unused_http_client(), "jev-latest", API_KEY);
    let requests = [
        JudgmentRequest {
            state: "  ".into(),
            questions: simple_request().questions,
        },
        JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::new(),
        },
        JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::from([(
                "choice".to_owned(),
                JudgmentQuestion::Choice {
                    instructions: "Choose".into(),
                    options: BTreeMap::new(),
                },
            )]),
        },
    ];

    for request in requests {
        let error = model
            .judge(&request)
            .await
            .expect_err("invalid request should fail locally");
        assert!(matches!(
            error,
            JudgmentError::EmptyState
                | JudgmentError::NoQuestions
                | JudgmentError::InvalidQuestion { .. }
        ));
        assert_no_api_key(&error);
    }
}

fn assert_provider_fields(error: &JudgmentError, expected_message: &str) {
    match error {
        JudgmentError::RateLimited {
            provider,
            model_id,
            message,
        }
        | JudgmentError::TransientProvider {
            provider,
            model_id,
            message,
        }
        | JudgmentError::Provider {
            provider,
            model_id,
            message,
        } => {
            assert_eq!(provider, "typesafe");
            assert_eq!(model_id, "jev-latest");
            assert_eq!(message, expected_message);
        }
        error => panic!("expected provider error, got {error}"),
    }
}

fn assert_no_api_key(error: &JudgmentError) {
    assert!(!error.to_string().contains(API_KEY));
    assert!(!format!("{error:?}").contains(API_KEY));
}

#[derive(Clone, Copy)]
enum ErrorKind {
    RateLimited,
    Transient,
}

impl ErrorKind {
    fn matches(self, error: &JudgmentError) -> bool {
        matches!(
            (self, error),
            (Self::RateLimited, JudgmentError::RateLimited { .. })
                | (Self::Transient, JudgmentError::TransientProvider { .. })
        )
    }
}
