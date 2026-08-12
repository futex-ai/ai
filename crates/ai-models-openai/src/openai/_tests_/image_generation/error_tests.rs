//! OpenAI image error classification tests.

use ai_interface::ImageGenerationError;
use serde_json::json;

use super::super::error::{classify_request_error, classify_status};

#[test]
fn retryable_statuses_have_distinct_rate_and_transient_classes() {
    assert!(matches!(
        classify_status(429, "gpt-image-2", &json!("limited")),
        ImageGenerationError::RateLimited { .. }
    ));

    for status in [408, 409, 425, 500] {
        assert!(matches!(
            classify_status(status, "gpt-image-2", &json!("retry")),
            ImageGenerationError::TransientProvider { .. }
        ));
    }
    assert!(matches!(
        classify_request_error(
            json_http::Error::transport("request timed out"),
            "gpt-image-2"
        ),
        ImageGenerationError::TransientProvider { .. }
    ));
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
        assert!(matches!(
            classify_status(400, "gpt-image-2", &body),
            ImageGenerationError::ContentPolicy { message, .. }
                if message == "blocked prompt"
        ));
    }
}

#[test]
fn unrecognized_client_failure_is_terminal_provider_error() {
    assert!(matches!(
        classify_status(
            400,
            "gpt-image-2",
            &json!({"error": {"code": "invalid_parameter", "message": "bad size"}})
        ),
        ImageGenerationError::Provider { message, .. } if message == "bad size"
    ));
}
