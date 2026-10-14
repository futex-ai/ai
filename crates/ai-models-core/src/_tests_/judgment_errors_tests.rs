//! Judgment status and request-error classification tests.

use ai_interface::JudgmentError;
use serde_json::{Value, json};

use super::{
    AUTH_HOOK_FAILED, classify_judgment_request_error, classify_judgment_status,
    decode_failure_body, http_status_message,
};

const SECRET: &str = "provider-secret-key";

#[test]
fn status_classes_follow_the_shared_http_classifier() {
    let secrets = [SECRET.to_owned()];
    let rate_limited = classify_judgment_status("openai", 429, "model", "slow down", &secrets);
    assert!(matches!(
        rate_limited,
        JudgmentError::RateLimited { provider, model_id, message }
            if provider == "openai" && model_id == "model" && message == "slow down"
    ));

    for status in [408, 409, 425, 500, 529, 599] {
        let error = classify_judgment_status("typesafe", status, "model", "busy", &secrets);
        assert!(
            matches!(error, JudgmentError::TransientProvider { .. }),
            "status {status}: {error}"
        );
    }

    for status in [400, 401, 403, 404, 422, 302] {
        let error = classify_judgment_status("typesafe", status, "model", "rejected", &secrets);
        assert!(
            matches!(error, JudgmentError::Provider { .. }),
            "status {status}: {error}"
        );
    }
}

#[test]
fn status_messages_are_redacted() {
    let error = classify_judgment_status(
        "openai",
        400,
        "model",
        "rejected Bearer provider-secret-key",
        &["Bearer provider-secret-key".to_owned(), SECRET.to_owned()],
    );

    let JudgmentError::Provider { message, .. } = &error else {
        panic!("expected a provider error, got {error}");
    };
    assert_eq!(message, "rejected [redacted]");
    assert!(!error.to_string().contains(SECRET));
}

#[test]
fn auth_errors_use_the_fixed_non_secret_diagnostic() {
    let error = classify_judgment_request_error(
        "typesafe",
        json_http::Error::auth("Authorization: Bearer provider-secret-key"),
        "jev-latest",
        &[],
    );

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
    assert_eq!(message, AUTH_HOOK_FAILED);
    assert!(!error.to_string().contains(SECRET));
    assert!(!format!("{error:?}").contains(SECRET));
}

#[test]
fn transport_errors_are_transient_and_redacted() {
    let secrets = [SECRET.to_owned()];
    let error = classify_judgment_request_error(
        "openai",
        json_http::Error::transport("connect failed for provider-secret-key"),
        "gpt-6-luna",
        &secrets,
    );

    let JudgmentError::TransientProvider { message, .. } = &error else {
        panic!("expected a transient provider error, got {error}");
    };
    assert!(message.contains("[redacted]"), "{message}");
    assert!(!message.contains(SECRET));
}

#[test]
fn local_request_failures_are_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON should fail");
    let error = classify_judgment_request_error(
        "openai",
        json_http::Error::SerializeRequest { source },
        "gpt-6-luna",
        &[],
    );

    assert!(matches!(error, JudgmentError::Internal(_)));
}

#[test]
fn response_shape_failures_are_terminal_provider_errors() {
    let error = classify_judgment_request_error(
        "openai",
        json_http::Error::HttpStatus {
            status: 418,
            body: json!({"error": {"message": "teapot provider-secret-key"}}),
        },
        "gpt-6-luna",
        &[SECRET.to_owned()],
    );

    let JudgmentError::Provider { message, .. } = &error else {
        panic!("expected a provider error, got {error}");
    };
    assert!(!message.contains(SECRET));
}

#[test]
fn failure_bodies_decode_as_json_text_or_status_fallback() {
    assert_eq!(
        decode_failure_body(400, br#"{"error":{"message":"bad"}}"#),
        json!({"error": {"message": "bad"}})
    );
    assert_eq!(
        decode_failure_body(400, b"plain text"),
        Value::String("plain text".to_owned())
    );
    assert_eq!(decode_failure_body(400, b"null"), Value::Null);
    assert_eq!(
        decode_failure_body(503, b""),
        Value::String("HTTP 503".to_owned())
    );
    assert_eq!(http_status_message(404), "HTTP 404");
}
