//! OpenAI Decisions authentication-hook redaction tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use ai_interface::{JudgmentError, JudgmentModel};
use json_http::{JsonHttpAuth, JsonHttpAuthMock, StaticHeaderAuth};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::super::test_support::{
    json_response, recording_http_client, simple_request, unused_http_client,
};
use super::OpenAiJudgmentModel;

const API_KEY: &str = "openai-secret-key";

#[tokio::test]
async fn bearer_hook_redacts_a_bare_token_from_provider_bodies() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"error": {"message": "rejected openai-secret-key"}}),
    ));
    let model = OpenAiJudgmentModel::with_auth(
        http_client,
        "gpt-6-luna",
        Arc::new(StaticHeaderAuth::bearer_token(API_KEY)),
    );

    let error = model
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    let JudgmentError::Provider { message, .. } = &error else {
        panic!("expected a provider error, got {error}");
    };
    assert_eq!(message, "rejected [redacted]");
    assert!(!error.to_string().contains(API_KEY));
    assert!(!format!("{error:?}").contains(API_KEY));
}

#[tokio::test]
async fn failing_auth_hook_uses_a_fixed_diagnostic_without_transport() {
    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth(
                "Authorization: Bearer openai-secret-key",
            ))),
    ));

    let error = OpenAiJudgmentModel::with_auth(unused_http_client(), "gpt-6-luna", auth)
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
    assert_eq!(provider, "openai");
    assert_eq!(model_id, "gpt-6-luna");
    assert_eq!(message, "authentication hook failed");
    assert!(!error.to_string().contains(API_KEY));
    assert!(!format!("{error:?}").contains(API_KEY));
}

#[tokio::test]
async fn rotating_auth_header_is_applied_once_sent_and_redacted() {
    let (http_client, requests) = recording_http_client(json_response(
        400,
        json!({"error": {"message": "rejected rotating-secret"}}),
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

    let error = OpenAiJudgmentModel::with_auth(http_client, "gpt-6-luna", auth)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    let JudgmentError::Provider { message, .. } = &error else {
        panic!("expected a provider error, got {error}");
    };
    assert_eq!(message, "rejected [redacted]");
    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer rotating-secret")
    );
}

#[tokio::test]
async fn custom_header_values_are_redacted_without_a_scheme() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"error": {"message": "rejected custom-secret"}}),
    ));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-Api-Key".to_owned(),
        "custom-secret".to_owned(),
    )])));

    let error = OpenAiJudgmentModel::with_auth(http_client, "gpt-6-luna", auth)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    let JudgmentError::Provider { message, .. } = &error else {
        panic!("expected a provider error, got {error}");
    };
    assert_eq!(message, "rejected [redacted]");
}
