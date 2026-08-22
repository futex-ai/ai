//! DeepSeek typed response-shape edge-case tests.

use ai_interface::ModelError;
use ai_models_core::ThinkingLevel;
use serde_json::json;

use crate::DEEPSEEK_V4_PRO;

use super::response;

#[test]
fn missing_or_empty_choices_are_provider_failures() {
    for body in [json!({}), json!({"choices": []})] {
        let error = complete(body).expect_err("missing first choice should fail");
        assert!(matches!(error, ModelError::Provider { .. }));
    }
}

#[test]
fn malformed_typed_response_fields_retain_internal_sources() {
    for body in [
        json!({"choices": "invalid"}),
        json!({"choices": [{}]}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"finish_reason": 7, "message": {"content": "Done"}}]}),
        json!({
            "choices": [{"message": {"content": "Done"}}],
            "usage": {"prompt_tokens": "many"}
        }),
        json!({
            "choices": [{"message": {"content": "Done"}}],
            "usage": {
                "completion_tokens_details": {"reasoning_tokens": "many"}
            }
        }),
    ] {
        let error = complete(body).expect_err("malformed typed response should fail");
        assert!(matches!(error, ModelError::Internal { .. }));
    }
}

fn complete(
    body: serde_json::Value,
) -> std::result::Result<ai_interface::ModelResponse, ModelError> {
    response::parse_response(
        DEEPSEEK_V4_PRO,
        DEEPSEEK_V4_PRO,
        ThinkingLevel::High,
        body,
        None,
    )
}
