//! TypeSafe authentication redaction tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use ai_interface::{JudgmentError, JudgmentModel};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, StaticHeaderAuth};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{
    TypeSafeJudgmentModel,
    test_support::{json_response, recording_http_client, simple_request, unused_http_client},
};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn bearer_hook_redacts_a_bare_token_from_provider_bodies() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"detail": {"message": "rejected typesafe-secret-key"}}),
    ));
    let model = TypeSafeJudgmentModel::with_auth(
        http_client,
        "jev-latest",
        Arc::new(StaticHeaderAuth::bearer_token(API_KEY)),
    );

    let error = model
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn custom_header_values_are_redacted_without_a_scheme() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"detail": {"message": "rejected custom-secret"}}),
    ));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-Api-Key".to_owned(),
        "custom-secret".to_owned(),
    )])));

    let error = TypeSafeJudgmentModel::with_auth(http_client, "jev-latest", auth)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, "custom-secret");
}

#[tokio::test]
async fn failing_auth_hook_uses_a_fixed_diagnostic_after_one_call() {
    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth(
                "Authorization: Bearer typesafe-secret-key",
            ))),
    ));

    let error = TypeSafeJudgmentModel::with_auth(unused_http_client(), "jev-latest", auth)
        .judge(&simple_request())
        .await
        .expect_err("auth-hook failure should return an error");

    let JudgmentError::TransientProvider {
        provider,
        model_id,
        message,
    } = &error
    else {
        panic!("expected a transient provider error, got {error}");
    };
    assert_eq!(provider, "typesafe");
    assert_eq!(model_id, "jev-latest");
    assert_eq!(message, "authentication hook failed");
    assert_not_exposed(&error, API_KEY);
}

#[tokio::test]
async fn rotating_auth_header_is_applied_once_sent_and_redacted() {
    let (http_client, requests) = recording_http_client(json_response(
        400,
        json!({"detail": {"message": "rejected rotating-secret"}}),
    ));
    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .answers_arc(Arc::new(|_, headers: &mut BTreeMap<String, String>| {
                headers.insert(
                    "Authorization".to_owned(),
                    "Bearer rotating-secret".to_owned(),
                );
                Ok(())
            })),
    ));

    let error = TypeSafeJudgmentModel::with_auth(http_client, "jev-latest", auth)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, "rotating-secret");
    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer rotating-secret")
    );
}

fn assert_redacted(error: &JudgmentError, secret: &str) {
    let display = error.to_string();
    let debug = format!("{error:?}");

    assert!(
        !display.contains(secret),
        "Display leaked {secret}: {display}"
    );
    assert!(!debug.contains(secret), "Debug leaked {secret}: {debug}");
    assert!(display.contains("[redacted]"));
    assert!(debug.contains("[redacted]"));
}

fn assert_not_exposed(error: &JudgmentError, secret: &str) {
    let display = error.to_string();
    let debug = format!("{error:?}");

    assert!(
        !display.contains(secret),
        "Display leaked {secret}: {display}"
    );
    assert!(!debug.contains(secret), "Debug leaked {secret}: {debug}");
}

#[tokio::test]
async fn overlapping_custom_header_values_leave_no_partial_credential() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"detail": {"message": "rejected token-secret"}}),
    ));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([
        ("X-Short".to_owned(), "token".to_owned()),
        ("X-Long".to_owned(), "token-secret".to_owned()),
    ])));

    let error = TypeSafeJudgmentModel::with_auth(http_client, "jev-latest", auth)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    let JudgmentError::Provider { message, .. } = &error else {
        panic!("expected a provider error, got {error}");
    };
    assert_eq!(message, "rejected [redacted]");
    assert_redacted(&error, "token-secret");
    assert_redacted(&error, "-secret");
}
