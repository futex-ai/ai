//! OpenAI video error classification tests.

use crate::openai::stream_support::error_context;

use ai_interface::VideoGenerationError;
use serde_json::json;

use super::super::error::{classify_request_error, classify_status};

#[test]
fn status_errors_have_typed_retry_and_policy_classes() {
    assert!(
        matches!(classify_status(&error_context("sora-2"), 429, &Default::default(), &json!({"error": {"message": "slow"}})), ref error if error.core().kind == ai_interface::ErrorKind::RateLimited)
    );
    assert!(
        matches!(classify_status(&error_context("sora-2"), 503, &Default::default(), &json!({"error": {"message": "down"}})), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
    for status in [408, 409, 425] {
        assert!(
            matches!(classify_status(&error_context("sora-2"), status, &Default::default(), &json!({"error": {"message": "retry"}})), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
        );
    }
    assert!(
        matches!(classify_status(&error_context("sora-2"), 400, &Default::default(), &json!({"error": {"code": "video_generation_safety_violation", "message": "blocked"}})), ref error if error.core().kind == ai_interface::ErrorKind::ContentPolicy)
    );
    assert!(
        matches!(classify_status(&error_context("sora-2"), 400, &Default::default(), &json!({"error": {"message": "bad"}})), ref error if error.core().kind == ai_interface::ErrorKind::InvalidRequest)
    );
    let context_limit = classify_status(
        &error_context("sora-2"),
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
fn transport_is_transient_and_serialization_is_internal() {
    assert!(
        matches!(classify_request_error(&error_context("sora-2"), json_http::Error::transport("offline")), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
    let source = serde_json::from_str::<u8>("not-json").unwrap_err();
    assert!(matches!(
        classify_request_error(&error_context("sora-2"), json_http::Error::DeserializeResponse {
                body: json!(null),
                source,
            }),
        VideoGenerationError::Internal { ref core, .. } if core.kind == ai_interface::ErrorKind::InvalidOutput
    ));
}
