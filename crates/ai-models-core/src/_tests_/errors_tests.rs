use ai_interface::ModelError;
use serde_json::json;

use crate::classify_json_http_error;

#[test]
fn classifies_structured_context_limit_errors() {
    let error = classify_json_http_error(
        "openai",
        "gpt",
        400,
        &json!({
            "error": {
                "code": "context_length_exceeded",
                "message": "request is too large"
            }
        }),
    );

    assert!(matches!(error, ModelError::ContextLimitExceeded { .. }));
}

#[test]
fn does_not_classify_broad_invalid_argument_as_context_limit() {
    let error = classify_json_http_error(
        "google",
        "gemini",
        400,
        &json!({
            "error": {
                "status": "INVALID_ARGUMENT",
                "message": "invalid schema"
            }
        }),
    );

    assert!(matches!(error, ModelError::Provider { .. }));
}
