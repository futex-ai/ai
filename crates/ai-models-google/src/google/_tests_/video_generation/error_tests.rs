//! Google video HTTP error classification tests.

use serde_json::json;

use crate::google::google_tests::stream_support::error_context;

use super::super::error::{classify_request_error, classify_status};

#[test]
fn status_errors_have_typed_retry_classes() {
    assert!(
        matches!(classify_status(&error_context("veo"), 429, &Default::default(), &json!({"error": {"message": "slow"}})), ref error if error.core().kind == ai_interface::ErrorKind::RateLimited)
    );
    assert!(
        matches!(classify_status(&error_context("veo"), 503, &Default::default(), &json!({"error": {"message": "down"}})), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
    for status in [408, 409, 425] {
        assert!(
            matches!(classify_status(&error_context("veo"), status, &Default::default(), &json!({"error": {"message": "retry"}})), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
        );
    }
    assert!(
        matches!(classify_status(&error_context("veo"), 400, &Default::default(), &json!({"error": {"message": "bad"}})), ref error if error.core().kind == ai_interface::ErrorKind::InvalidRequest)
    );
}

#[test]
fn transport_failure_is_transient() {
    assert!(
        matches!(classify_request_error(&error_context("veo"), json_http::Error::transport("offline")), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
}
