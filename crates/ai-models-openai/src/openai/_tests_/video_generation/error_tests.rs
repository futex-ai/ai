//! OpenAI video error classification tests.

use ai_interface::VideoGenerationError;
use serde_json::json;

use super::super::error::{classify_request_error, classify_status};

#[test]
fn status_errors_have_typed_retry_and_policy_classes() {
    assert!(matches!(
        classify_status(429, "sora-2", &json!({"error": {"message": "slow"}})),
        VideoGenerationError::RateLimited { .. }
    ));
    assert!(matches!(
        classify_status(503, "sora-2", &json!({"error": {"message": "down"}})),
        VideoGenerationError::TransientProvider { .. }
    ));
    for status in [408, 409, 425] {
        assert!(matches!(
            classify_status(status, "sora-2", &json!({"error": {"message": "retry"}})),
            VideoGenerationError::TransientProvider { .. }
        ));
    }
    assert!(matches!(
        classify_status(
            400,
            "sora-2",
            &json!({"error": {"code": "video_generation_safety_violation", "message": "blocked"}})
        ),
        VideoGenerationError::ContentPolicy { .. }
    ));
    assert!(matches!(
        classify_status(400, "sora-2", &json!({"error": {"message": "bad"}})),
        VideoGenerationError::Provider { .. }
    ));
}

#[test]
fn transport_is_transient_and_serialization_is_internal() {
    assert!(matches!(
        classify_request_error(json_http::Error::transport("offline"), "sora-2"),
        VideoGenerationError::TransientProvider { .. }
    ));
    let source = serde_json::from_str::<u8>("not-json").unwrap_err();
    assert!(matches!(
        classify_request_error(
            json_http::Error::DeserializeResponse {
                body: json!(null),
                source,
            },
            "sora-2"
        ),
        VideoGenerationError::Internal(_)
    ));
}
