//! DeepSeek normalized text-response tests.

use ai_interface::{FinishReason, ModelError, ModelUsage};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use crate::{DEEPSEEK_V4_FLASH, DEEPSEEK_V4_FLASH_THINKING_DISABLED};

use super::response;

#[test]
fn maps_stopped_text_and_selected_model_metadata() {
    let response = complete(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {"content": "Done"}
        }]
    }))
    .expect("stopped response should parse");

    assert_eq!(response.provider, "deepseek");
    assert_eq!(response.model_id, DEEPSEEK_V4_FLASH);
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some(DEEPSEEK_V4_FLASH_THINKING_DISABLED)
    );
    assert_eq!(response.thinking_level.as_deref(), Some("disabled"));
    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert!(response.tool_calls.is_empty());
    assert!(response.provider_context.is_empty());
    assert_eq!(response.usage, ModelUsage::default());
}

#[test]
fn normalizes_nullable_content_and_rejects_missing_choices() {
    let response = complete(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {"content": null}
        }]
    }))
    .expect("nullable content should normalize");
    assert_eq!(response.assistant_message, "");

    let error = complete(json!({"choices": []})).expect_err("missing choices should fail");
    assert!(matches!(error, ModelError::Provider { .. }));
}

#[test]
fn retains_typed_deserialization_sources_as_internal_errors() {
    for malformed in [
        json!({"choices": "invalid"}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"finish_reason": 7, "message": {"content": "Done"}}]}),
    ] {
        let error = complete(malformed).expect_err("malformed typed response should fail");
        assert!(matches!(error, ModelError::Internal { .. }));
    }
}

fn complete(
    body: serde_json::Value,
) -> std::result::Result<ai_interface::ModelResponse, ModelError> {
    response::parse_response(
        DEEPSEEK_V4_FLASH_THINKING_DISABLED,
        DEEPSEEK_V4_FLASH,
        ThinkingLevel::Disabled,
        body,
        None,
    )
}
