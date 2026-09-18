//! TypeSafe judgment credential-redaction tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use ai_interface::{JudgmentError, JudgmentModel};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{
    TypeSafeJudgmentModel,
    test_support::{
        json_response, recording_http_client, simple_request, transport_failure_http_client,
        unused_http_client,
    },
};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn bearer_key_is_redacted_from_transport_diagnostics() {
    let error = TypeSafeJudgmentModel::new(
        transport_failure_http_client("connection failed for Bearer typesafe-secret-key"),
        "jev-latest",
        API_KEY,
    )
    .judge(&simple_request())
    .await
    .expect_err("transport failure should return an error");

    assert!(matches!(error, JudgmentError::TransientProvider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn bearer_key_is_redacted_from_provider_bodies() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"detail": {"message": "rejected Bearer typesafe-secret-key"}}),
    ));

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn custom_header_values_are_redacted_from_provider_bodies() {
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
async fn auth_hook_diagnostics_are_redacted() {
    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new((
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth(
                "Authorization: Bearer typesafe-secret-key",
            ))),
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth(
                "Authorization: Bearer typesafe-secret-key",
            ))),
    )));
    let mut model = TypeSafeJudgmentModel::new(unused_http_client(), "jev-latest", API_KEY);
    model.auth = auth;

    let error = model
        .judge(&simple_request())
        .await
        .expect_err("auth-hook failure should return an error");

    assert!(matches!(error, JudgmentError::TransientProvider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn malformed_payload_diagnostics_are_redacted() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: br#"{
            "model":"jev-1.13.0",
            "answers":{"is_urgent":{"type":"typesafe-secret-key"}}
        }"#
        .to_vec(),
    });

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("malformed response should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, API_KEY);
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
