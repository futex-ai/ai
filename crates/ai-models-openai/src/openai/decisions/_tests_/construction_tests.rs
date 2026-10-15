//! OpenAI judgment model construction tests.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ai_interface::{JudgmentError, JudgmentModel};
use json_http::{JsonHttpBody, StaticHeaderAuth};
use serde_json::json;

use super::super::test_support::{
    json_response, recording_http_client, simple_request, successful_response, unused_http_client,
};
use super::OpenAiJudgmentModel;

#[test]
fn constructors_use_the_documented_defaults_and_clone() {
    let keyed = OpenAiJudgmentModel::new(unused_http_client(), "gpt-6-luna", "openai-key");

    assert_eq!(keyed.transport.provider(), "openai");
    assert_eq!(keyed.transport.model_id(), "gpt-6-luna");
    assert_eq!(
        keyed.transport.endpoint(),
        "https://api.openai.com/v1/decisions"
    );
    assert_eq!(keyed.transport.timeout(), Duration::from_secs(60));

    let authenticated = OpenAiJudgmentModel::with_auth(
        unused_http_client(),
        "gpt-6-luna",
        Arc::new(StaticHeaderAuth::default()),
    );
    assert_eq!(authenticated.transport.model_id(), "gpt-6-luna");
    let _clone = authenticated.clone();
}

/// The bearer header yields only `Bearer openai split-key` and `split-key`, so
/// only the explicit secret that `new` registers redacts the whole key.
#[tokio::test]
async fn api_key_constructor_redacts_the_whole_key_and_auth_hooks_add_no_secret() {
    let rejection = json!({"error": {"message": "rejected openai split-key"}});
    let (http_client, _) = recording_http_client(json_response(400, rejection.clone()));
    let keyed = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", "openai split-key")
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");
    assert!(
        matches!(&keyed, JudgmentError::Provider { message, .. } if message == "rejected [redacted]"),
        "{keyed}"
    );

    let (http_client, _) = recording_http_client(json_response(400, rejection));
    let hooked = OpenAiJudgmentModel::with_auth(
        http_client,
        "gpt-6-luna",
        Arc::new(StaticHeaderAuth::default()),
    )
    .judge(&simple_request())
    .await
    .expect_err("provider rejection should return an error");
    assert!(
        matches!(&hooked, JudgmentError::Provider { message, .. } if message == "rejected openai split-key"),
        "{hooked}"
    );
}

#[tokio::test]
async fn auth_endpoint_timeout_and_unlisted_model_are_passed_through() {
    let (http_client, requests) = recording_http_client(successful_response());
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-Test-Auth".to_owned(),
        "injected".to_owned(),
    )])));
    let model = OpenAiJudgmentModel::with_auth(http_client, "gpt-7-luna", auth)
        .with_endpoint("https://openai.test/decisions")
        .with_timeout(Duration::from_secs(7));

    model
        .judge(&simple_request())
        .await
        .expect("unlisted provider model should be accepted");

    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests[0].url, "https://openai.test/decisions");
    assert_eq!(requests[0].timeout, Duration::from_secs(7));
    assert_eq!(
        requests[0].headers.get("X-Test-Auth").map(String::as_str),
        Some("injected")
    );
    assert_eq!(
        requests[0]
            .body
            .as_ref()
            .and_then(JsonHttpBody::as_json)
            .expect("request body should be JSON")["model"],
        "gpt-7-luna"
    );
}
