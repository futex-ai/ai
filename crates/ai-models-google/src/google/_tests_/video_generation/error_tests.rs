//! Google video HTTP error classification tests.

use ai_interface::VideoGenerationError;
use serde_json::json;

use super::super::error::{classify_request_error, classify_status};

#[test]
fn status_errors_have_typed_retry_classes() {
    assert!(matches!(
        classify_status(429, "veo", &json!({"error": {"message": "slow"}})),
        VideoGenerationError::RateLimited { .. }
    ));
    assert!(matches!(
        classify_status(503, "veo", &json!({"error": {"message": "down"}})),
        VideoGenerationError::TransientProvider { .. }
    ));
    for status in [408, 409, 425] {
        assert!(matches!(
            classify_status(status, "veo", &json!({"error": {"message": "retry"}})),
            VideoGenerationError::TransientProvider { .. }
        ));
    }
    assert!(matches!(
        classify_status(400, "veo", &json!({"error": {"message": "bad"}})),
        VideoGenerationError::Provider { .. }
    ));
}

#[test]
fn transport_failure_is_transient() {
    assert!(matches!(
        classify_request_error(json_http::Error::transport("offline"), "veo"),
        VideoGenerationError::TransientProvider { .. }
    ));
}
