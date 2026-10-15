//! Judgment transport failure classification and redaction tests.

use std::sync::Arc;

use ai_interface::JudgmentError;
use json_http::{JsonHttpAuth, JsonHttpAuthMock};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::judgment_transport_support::{
    MODEL_ID, PROVIDER, TEST_ENDPOINT, TOKEN, bearer_transport, bytes_response, expect_error,
    failing_http_client, provider_fields, recording_http_client, unused_http_client,
};
use super::{AUTH_HOOK_FAILED, HttpJudgmentTransport, JudgmentTransport};

#[derive(Clone, Copy, Debug)]
enum Class {
    RateLimited,
    Transient,
    Terminal,
}

impl Class {
    fn matches(self, error: &JudgmentError) -> bool {
        matches!(
            (self, error),
            (Self::RateLimited, JudgmentError::RateLimited { .. })
                | (Self::Transient, JudgmentError::TransientProvider { .. })
                | (Self::Terminal, JudgmentError::Provider { .. })
        )
    }
}

#[tokio::test]
async fn failure_statuses_classify_the_extracted_message_and_redact_it() {
    let cases = [
        (429, Class::RateLimited),
        (408, Class::Transient),
        (409, Class::Transient),
        (425, Class::Transient),
        (500, Class::Transient),
        (503, Class::Transient),
        (529, Class::Transient),
        (599, Class::Transient),
        (400, Class::Terminal),
        (401, Class::Terminal),
        (403, Class::Terminal),
        (404, Class::Terminal),
        (422, Class::Terminal),
    ];
    for (status, class) in cases {
        let body = json!({
            "failure": {"text": "rejected Bearer core-secret, explicit-key, core-secret"},
            "other": "ignored by the extractor"
        });
        let (http_client, _) = recording_http_client(bytes_response(status, &to_bytes(&body)));

        let error = expect_error(
            bearer_transport(http_client)
                .with_explicit_secret("explicit-key")
                .post(json!({}))
                .await,
        );

        assert!(
            class.matches(&error),
            "status {status} as {class:?}: {error}"
        );
        assert_eq!(
            provider_fields(&error),
            (
                PROVIDER,
                MODEL_ID,
                "rejected [redacted], [redacted], [redacted]"
            ),
            "status {status}"
        );
        assert!(!format!("{error:?}").contains(TOKEN));
    }
}

#[tokio::test]
async fn bodies_without_the_extractor_field_keep_text_or_compact_json() {
    let cases = [
        (to_bytes(&json!("plain rejection")), "plain rejection"),
        (
            to_bytes(&json!({"code": "bad_request"})),
            "{\"code\":\"bad_request\"}",
        ),
        (to_bytes(&Value::Null), "null"),
        (b"not json".to_vec(), "not json"),
        (Vec::new(), "HTTP 400"),
    ];
    for (body, expected) in cases {
        let (http_client, _) = recording_http_client(bytes_response(400, &body));

        let error = expect_error(bearer_transport(http_client).post(json!({})).await);

        assert!(matches!(error, JudgmentError::Provider { .. }), "{error}");
        assert_eq!(provider_fields(&error).2, expected);
    }
}

#[tokio::test]
async fn empty_failure_bodies_report_the_status_line() {
    for (status, class) in [(503, Class::Transient), (429, Class::RateLimited)] {
        let (http_client, _) = recording_http_client(bytes_response(status, b""));

        let error = expect_error(bearer_transport(http_client).post(json!({})).await);

        assert!(class.matches(&error), "status {status}: {error}");
        assert_eq!(
            provider_fields(&error).2,
            ["HTTP ", &status.to_string()].concat()
        );
    }
}

#[tokio::test]
async fn statuses_outside_2xx_are_terminal_even_with_a_valid_body() {
    for status in [199, 300, 302, 304] {
        let body = json!({"failure": {"text": "moved"}, "answers": []});
        let (http_client, _) = recording_http_client(bytes_response(status, &to_bytes(&body)));

        let error = expect_error(bearer_transport(http_client).post(json!({})).await);

        assert!(
            matches!(error, JudgmentError::Provider { .. }),
            "status {status}: {error}"
        );
        assert_eq!(provider_fields(&error).2, "moved", "status {status}");
    }
}

#[tokio::test]
async fn transport_failures_are_transient_and_redacted() {
    let http_client = failing_http_client(json_http::Error::transport(
        "connect failed for Authorization: Bearer core-secret",
    ));

    let error = expect_error(bearer_transport(http_client).post(json!({})).await);

    assert!(
        matches!(error, JudgmentError::TransientProvider { .. }),
        "{error}"
    );
    let (provider, model_id, message) = provider_fields(&error);
    assert_eq!((provider, model_id), (PROVIDER, MODEL_ID));
    assert!(message.contains("[redacted]"), "{message}");
    assert!(!message.contains(TOKEN), "{message}");
    assert!(!format!("{error:?}").contains(TOKEN));
}

#[tokio::test]
async fn local_send_failures_are_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON should fail");
    let http_client = failing_http_client(json_http::Error::SerializeRequest { source });

    let error = expect_error(bearer_transport(http_client).post(json!({})).await);

    assert!(matches!(error, JudgmentError::Internal(_)), "{error}");
}

#[tokio::test]
async fn failing_auth_hook_uses_the_fixed_diagnostic_without_a_transport_call() {
    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth(
                "Authorization: Bearer core-secret",
            ))),
    ));

    let error = expect_error(
        HttpJudgmentTransport::new(TEST_ENDPOINT, MODEL_ID, unused_http_client(), auth)
            .with_explicit_secret("explicit-key")
            .post(json!({}))
            .await,
    );

    assert!(
        matches!(error, JudgmentError::TransientProvider { .. }),
        "{error}"
    );
    assert_eq!(
        provider_fields(&error),
        (PROVIDER, MODEL_ID, AUTH_HOOK_FAILED)
    );
    assert!(!error.to_string().contains(TOKEN));
    assert!(!format!("{error:?}").contains(TOKEN));
}

fn to_bytes(body: &Value) -> Vec<u8> {
    serde_json::to_vec(body).expect("JSON fixture should serialize")
}
