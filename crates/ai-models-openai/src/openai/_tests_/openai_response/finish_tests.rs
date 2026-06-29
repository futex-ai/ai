//! Finish reason and tool-call response tests.

use ai_interface::{FinishReason, ModelError};
use serde_json::json;

use super::{ThinkingLevel, openai_function_call_body, openai_text_body, parse_response};

#[test]
fn maps_response_finish_reasons() {
    let cases = [
        (
            openai_text_body("Done"),
            FinishReason::Stop,
            "completed response should stop",
        ),
        (
            openai_function_call_body("{\"path\":\"root\"}"),
            FinishReason::ToolCalls,
            "function call output item should request tools",
        ),
        (
            json!({
                "status": "incomplete",
                "incomplete_details": { "reason": "max_output_tokens" },
                "output": []
            }),
            FinishReason::Truncated,
            "max output tokens should truncate",
        ),
        (
            json!({
                "status": "incomplete",
                "incomplete_details": { "reason": "content_filter" },
                "output": []
            }),
            FinishReason::Filtered,
            "content filter should filter",
        ),
        (
            json!({
                "status": "queued",
                "output": []
            }),
            FinishReason::Other("queued".to_owned()),
            "unknown status should be retained",
        ),
    ];

    for (body, expected, label) in cases {
        let response =
            parse_response("gpt-5.5", "gpt-5.5", ThinkingLevel::Medium, body, None).expect(label);
        assert_eq!(response.finish_reason, expected, "{label}");
    }
}

#[test]
fn maps_refusal_content_to_filtered_finish_reason() {
    let response = parse_response(
        "gpt-5.5",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "refusal",
                    "refusal": "I cannot help with that."
                }]
            }]
        }),
        None,
    )
    .expect("refusal response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert_eq!(response.assistant_message, "I cannot help with that.");
}

#[test]
fn parses_multiple_function_call_output_items() {
    let response = parse_response(
        "gpt-5.5",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                },
                {
                    "type": "function_call",
                    "call_id": "call_2",
                    "name": "memory_log",
                    "arguments": "{\"path\":\"root\",\"content\":\"done\"}"
                }
            ]
        }),
        None,
    )
    .expect("function calls should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 2);
    assert_eq!(response.tool_calls[0].id, "call_1");
    assert_eq!(response.tool_calls[1].id, "call_2");
}

#[test]
fn incomplete_function_call_response_is_not_dispatchable() {
    let response = parse_response(
        "gpt-5.5",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "incomplete",
            "incomplete_details": { "reason": "max_output_tokens" },
            "output": [{
                "type": "function_call",
                "call_id": "call_1",
                "name": "memory_read",
                "arguments": "{\"path\":\"root\"}"
            }]
        }),
        None,
    )
    .expect("incomplete function-call response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert!(response.tool_calls.is_empty());
}

#[test]
fn rejects_invalid_function_call_arguments() {
    let error = parse_response(
        "gpt-5.5",
        "gpt-5.5",
        ThinkingLevel::Medium,
        openai_function_call_body("not json"),
        None,
    )
    .expect_err("invalid tool arguments should fail");

    assert!(matches!(error, ModelError::Provider { .. }));
}
