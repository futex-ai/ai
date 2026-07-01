//! Tests for structured-output terminal finish handling.

use ai_interface::{FinishReason, StructuredOutputSchema};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::response::parse_response;

#[test]
fn structured_output_refusal_preserves_filtered_finish_reason() {
    let response = parse_response(
        "claude-sonnet-4-6",
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        json!({
            "stop_reason": "refusal",
            "content": [
                { "type": "text", "text": "I cannot provide that." }
            ]
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
        "claude-sonnet-4-6",
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        json!({
            "stop_reason": "max_tokens",
            "content": [
                { "type": "text", "text": "{\"summary\":" }
            ]
        }),
        Some(&status_schema()),
    )
    .expect("truncated structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.assistant_message, "{\"summary\":");
}

#[test]
fn terminal_response_with_tool_use_suppresses_tool_calls() {
    let response = parse_response(
        "claude-sonnet-4-6",
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        json!({
            "stop_reason": "max_tokens",
            "content": [{
                "type": "tool_use",
                "id": "call_1",
                "name": "memory_read",
                "input": { "path": "root" }
            }],
            "usage": {
                "input_tokens": 8,
                "output_tokens": 4
            }
        }),
        None,
    )
    .expect("terminal Anthropic response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert!(response.tool_calls.is_empty());
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
