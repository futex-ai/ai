//! DeepSeek normalized text-response tests.

use ai_interface::ErrorKind;

use ai_interface::{Effort, FinishReason, ModelError, ModelUsage};
use serde_json::json;

use crate::part_test_support::replay_items;

use crate::deepseek::DEEPSEEK_V4_FLASH;

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
    assert_eq!(
        response.model_id,
        DEEPSEEK_V4_FLASH.provider_model_id.as_ref()
    );
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("host-deepseek-routing")
    );
    assert_eq!(response.effort().map(Effort::as_str), Some("disabled"));
    assert_eq!(response.text(), "Done");
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert!(response.tool_calls().is_empty());
    assert!(replay_items(&response).is_empty());
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
    assert_eq!(response.text(), "");

    let error = complete(json!({"choices": []})).expect_err("missing choices should fail");
    assert!(matches!(error.core().kind, ErrorKind::InvalidOutput));
}

#[test]
fn retains_typed_deserialization_sources_as_internal_errors() {
    for malformed in [
        json!({"choices": "invalid"}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"finish_reason": 7, "message": {"content": "Done"}}]}),
    ] {
        let error = complete(malformed).expect_err("malformed typed response should fail");
        assert!(
            matches!(error, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::InvalidOutput)
        );
    }
}

fn complete(
    body: serde_json::Value,
) -> std::result::Result<ai_interface::ModelResponse, ModelError> {
    response::parse_response(
        "host-deepseek-routing",
        DEEPSEEK_V4_FLASH.provider_model_id.as_ref(),
        Effort::Disabled,
        body,
        None,
    )
}
