//! Tests for structured-output terminal finish handling.

use ai_interface::{FinishReason, StructuredOutputSchema};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::super::response::parse_response;

#[test]
fn structured_output_filter_preserves_filtered_finish_reason() {
    let response = parse_response(
        "gemini-2.5-pro",
        "gemini-2.5-pro",
        ThinkingLevel::Disabled,
        json!({
            "candidates": [{
                "finishReason": "SAFETY"
            }]
        }),
        Some(&status_schema()),
    )
    .expect("filtered structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert_eq!(response.structured_output, None);
    assert!(response.assistant_message.is_empty());
}

#[test]
fn structured_output_truncation_preserves_truncated_finish_reason() {
    let response = parse_response(
        "gemini-2.5-pro",
        "gemini-2.5-pro",
        ThinkingLevel::Disabled,
        json!({
            "candidates": [{
                "finishReason": "MAX_TOKENS",
                "content": {
                    "parts": [{ "text": "{\"summary\":" }]
                }
            }]
        }),
        Some(&status_schema()),
    )
    .expect("truncated structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.assistant_message, "{\"summary\":");
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "summary": {"type": "string"},
                "done": {"type": "boolean"}
            },
            "required": ["summary", "done"]
        }),
    }
}
