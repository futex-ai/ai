//! OpenAI image error classification tests.

use ai_interface::ImageGenerationError;
use reqwest::StatusCode;

use super::super::error::{classify_status, request_error};

#[test]
fn retryable_statuses_have_distinct_rate_and_transient_classes() {
    assert!(matches!(
        classify_status(StatusCode::TOO_MANY_REQUESTS, "gpt-image-2", "limited"),
        ImageGenerationError::RateLimited { .. }
    ));

    for status in [
        StatusCode::REQUEST_TIMEOUT,
        StatusCode::CONFLICT,
        StatusCode::from_u16(425).unwrap(),
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        assert!(matches!(
            classify_status(status, "gpt-image-2", "retry"),
            ImageGenerationError::TransientProvider { .. }
        ));
    }
    assert!(matches!(
        request_error("gpt-image-2", "request timed out"),
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
        let body = format!("{{\"error\":{{\"code\":\"{code}\",\"message\":\"blocked prompt\"}}}}");
        assert!(matches!(
            classify_status(StatusCode::BAD_REQUEST, "gpt-image-2", &body),
            ImageGenerationError::ContentPolicy { message, .. }
                if message == "blocked prompt"
        ));
    }
}

#[test]
fn unrecognized_client_failure_is_terminal_provider_error() {
    assert!(matches!(
        classify_status(
            StatusCode::BAD_REQUEST,
            "gpt-image-2",
            "{\"error\":{\"code\":\"invalid_parameter\",\"message\":\"bad size\"}}"
        ),
        ImageGenerationError::Provider { message, .. } if message == "bad size"
    ));
}
