//! Judgment transport request, authentication, and success-path tests.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ai_interface::JudgmentError;
use json_http::{JsonHttpAuth, JsonHttpAuthMock, JsonHttpBody, JsonHttpMethod, StaticHeaderAuth};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::judgment_transport_support::{
    MODEL_ID, PROVIDER, TEST_ENDPOINT, bearer_transport, bytes_response, recording_http_client,
    unused_http_client,
};
use super::{
    HttpJudgmentTransport, JudgmentHttpResponse, JudgmentTransport, JudgmentTransportMock,
    malformed_judgment_payload,
};

#[test]
fn construction_uses_the_endpoint_defaults_and_overrides_replace_them() {
    let transport = bearer_transport(unused_http_client());

    assert_eq!(transport.provider(), PROVIDER);
    assert_eq!(transport.model_id(), MODEL_ID);
    assert_eq!(transport.endpoint(), "https://judge.test/v1/judge");
    assert_eq!(transport.timeout(), Duration::from_secs(45));

    let overridden = transport
        .clone()
        .with_endpoint("https://judge.example/alternate")
        .with_timeout(Duration::from_secs(5));
    assert_eq!(overridden.provider(), PROVIDER);
    assert_eq!(overridden.model_id(), MODEL_ID);
    assert_eq!(overridden.endpoint(), "https://judge.example/alternate");
    assert_eq!(overridden.timeout(), Duration::from_secs(5));
    assert_eq!(transport.endpoint(), "https://judge.test/v1/judge");
}

#[tokio::test]
async fn posts_the_exact_json_body_to_the_configured_endpoint() {
    let (http_client, requests) = recording_http_client(bytes_response(200, b"{}"));
    let body = json!({"model": MODEL_ID, "questions": [{"name": "urgent"}]});

    bearer_transport(http_client)
        .with_endpoint("https://judge.example/alternate")
        .with_timeout(Duration::from_secs(5))
        .post(body.clone())
        .await
        .expect("a 2xx response should succeed");

    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
    assert_eq!(requests[0].url, "https://judge.example/alternate");
    assert_eq!(requests[0].timeout, Duration::from_secs(5));
    assert_eq!(requests[0].body, Some(JsonHttpBody::Json(body)));
}

#[tokio::test]
async fn auth_hook_runs_once_on_empty_headers_and_its_headers_are_sent_unchanged() {
    let (http_client, requests) = recording_http_client(bytes_response(200, b"{}"));
    let applied = BTreeMap::from([
        (
            "Authorization".to_owned(),
            "Bearer rotating-token".to_owned(),
        ),
        ("X-Trace".to_owned(), "trace-1".to_owned()),
    ]);
    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .answers_arc({
                let applied = applied.clone();
                Arc::new(move |_, headers: &mut BTreeMap<String, String>| {
                    assert!(headers.is_empty(), "the hook must start from no headers");
                    headers.extend(applied.clone());
                    Ok(())
                })
            }),
    ));

    HttpJudgmentTransport::new(TEST_ENDPOINT, MODEL_ID, http_client, auth)
        .post(json!({}))
        .await
        .expect("a 2xx response should succeed");

    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].headers, applied);
}

#[tokio::test]
async fn redaction_set_holds_explicit_secrets_header_values_and_scheme_tokens() {
    let (http_client, _) = recording_http_client(bytes_response(200, b"{}"));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([
        ("Authorization".to_owned(), "Bearer header-token".to_owned()),
        ("X-Api-Key".to_owned(), "custom-key".to_owned()),
    ])));

    let response = HttpJudgmentTransport::new(TEST_ENDPOINT, MODEL_ID, http_client, auth)
        .with_explicit_secret("explicit-key")
        .post(json!({}))
        .await
        .expect("a 2xx response should succeed");

    assert_eq!(
        response.secrets,
        vec![
            "explicit-key".to_owned(),
            "Bearer header-token".to_owned(),
            "header-token".to_owned(),
            "custom-key".to_owned(),
        ]
    );
}

#[tokio::test]
async fn every_2xx_status_returns_the_raw_body_bytes_without_parsing() {
    for status in [200, 201, 204, 299] {
        let (http_client, _) = recording_http_client(bytes_response(status, b"not json"));

        let response = bearer_transport(http_client)
            .post(json!({}))
            .await
            .expect("a 2xx response should succeed");

        assert_eq!(response.body, b"not json".to_vec(), "status {status}");
    }
}

#[test]
fn malformed_payload_errors_use_the_fixed_prefix_and_keep_identity() {
    let error = malformed_judgment_payload("openai", "gpt-6-luna", &"missing field `answers`");

    let JudgmentError::Provider {
        provider,
        model_id,
        message,
    } = &error
    else {
        panic!("expected a terminal provider error, got {error}");
    };
    assert_eq!(provider, "openai");
    assert_eq!(model_id, "gpt-6-luna");
    assert_eq!(
        message,
        "malformed provider payload: missing field `answers`"
    );
}

#[tokio::test]
async fn the_generated_mock_stands_in_for_the_transport_trait() {
    let transport: Arc<dyn JudgmentTransport> = Arc::new(Unimock::new(
        JudgmentTransportMock::post
            .next_call(matching!(_))
            .returns(Ok(JudgmentHttpResponse {
                body: b"{}".to_vec(),
                secrets: vec!["mock-secret".to_owned()],
            })),
    ));

    let response = transport
        .post(json!({"model": MODEL_ID}))
        .await
        .expect("the mocked transport should succeed");

    assert_eq!(response.body, b"{}".to_vec());
    assert_eq!(response.secrets, vec!["mock-secret".to_owned()]);
}
