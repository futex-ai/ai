//! OpenAI image error classification tests.

use crate::openai::stream_support::error_context;

use serde_json::json;

use super::super::error::{classify_request_error, classify_status};

#[test]
fn retryable_statuses_have_distinct_rate_and_transient_classes() {
    assert!(
        matches!(classify_status(&error_context("gpt-image-2"), 429, &Default::default(), &json!("limited")), ref error if error.core().kind == ai_interface::ErrorKind::RateLimited)
    );

    for status in [408, 409, 425, 500] {
        assert!(
            matches!(classify_status(&error_context("gpt-image-2"), status, &Default::default(), &json!("retry")), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
        );
    }
    assert!(
        matches!(classify_request_error(&error_context("gpt-image-2"), json_http::Error::transport("request timed out")), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
}

#[test]
fn documented_safety_codes_are_content_policy_refusals() {
    for code in [
        "content_policy_violation",
        "moderation_blocked",
        "safety_violation",
        "image_generation_safety_violation",
    ] {
        let body = json!({"error": {"code": code, "message": "blocked prompt"}});
        assert!(
            matches!(classify_status(&error_context("gpt-image-2"), 400, &Default::default(), &body), ref error if error.core().kind == ai_interface::ErrorKind::ContentPolicy && (error.core().message == "blocked prompt"))
        );
    }
    let context_limit = classify_status(
        &error_context("gpt-image-2"),
        400,
        &Default::default(),
        &json!({"error":{"code":"context_length_exceeded","type":"content_policy_violation","message":"too many tokens"}}),
    );
    assert_eq!(
        context_limit.core().kind,
        ai_interface::ErrorKind::ContextLimitExceeded
    );
    assert_eq!(context_limit.core().message, "too many tokens");
}

#[test]
fn unrecognized_client_failure_is_terminal_provider_error() {
    assert!(
        matches!(classify_status(&error_context("gpt-image-2"), 400, &Default::default(), &json!({"error": {"code": "invalid_parameter", "message": "bad size"}})), ref error if error.core().kind == ai_interface::ErrorKind::InvalidRequest && (error.core().message == "bad size"))
    );
}
