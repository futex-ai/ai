//! Tests for structured-output terminal finish handling.

use ai_interface::{FinishReason, StructuredOutputSchema};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::response::parse_response;

#[test]
fn structured_output_filter_preserves_filtered_finish_reason() {
    let response = parse_response(
        "grok-4",
        "grok-4",
        ThinkingLevel::Disabled,
        "test_scope",
        json!({
            "choices": [{
                "finish_reason": "content_filter",
                "message": {
                    "content": "I cannot provide that.",
                    "tool_calls": []
                }
            }]
        }),
        Some(&status_schema()),
    )
    .expect("filtered structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.assistant_message, "I cannot provide that.");
}

#[test]
fn structured_output_truncation_preserves_truncated_finish_reason() {
    let response = parse_response(
        "grok-4",
        "grok-4",
        ThinkingLevel::Disabled,
        "test_scope",
        json!({
            "choices": [{
                "finish_reason": "length",
                "message": {
                    "content": "{\"summary\":",
                    "tool_calls": []
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
