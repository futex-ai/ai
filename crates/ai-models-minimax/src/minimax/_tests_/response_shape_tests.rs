//! MiniMax response-shape edge-case tests.

use ai_interface::{ModelError, ModelUsage};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use crate::MINIMAX_M3;

use super::response;

#[test]
fn rejects_empty_choices_and_malformed_typed_fields() {
    let empty_error = parse(json!({"choices": []})).expect_err("empty choices should fail");
    assert!(matches!(empty_error, ModelError::Provider { .. }));

    for malformed in [
        json!({"choices": "invalid"}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"message": {"reasoning_details": [{"index": "zero"}]}}]}),
    ] {
        let error = parse(malformed).expect_err("malformed typed response should fail");
        assert!(matches!(error, ModelError::Internal { .. }));
    }
}

#[test]
fn accepts_null_or_empty_content_and_absent_usage() {
    for content in [serde_json::Value::Null, json!("")] {
        let response = parse(json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": content}
            }]
        }))
        .expect("nullable content should normalize");

        assert_eq!(response.assistant_message, "");
        assert_eq!(response.usage, ModelUsage::default());
    }
}

fn parse(body: serde_json::Value) -> std::result::Result<ai_interface::ModelResponse, ModelError> {
    response::parse_response(MINIMAX_M3, MINIMAX_M3, ThinkingLevel::Medium, body, None)
}
