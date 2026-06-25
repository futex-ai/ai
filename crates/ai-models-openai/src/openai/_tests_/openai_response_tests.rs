//! OpenAI Responses response parser edge cases.

use ai_interface::{FinishReason, ModelError, OpenAiReasoningSummary, ProviderConversationItem};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::response::parse_response;

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

#[test]
fn ignores_reasoning_output_items() {
    let response = parse_response(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        ThinkingLevel::ExtraHigh,
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_hidden",
                    "summary": [{ "type": "summary_text", "text": "hidden" }]
                },
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "Visible" }]
                },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            ]
        }),
        None,
    )
    .expect("mixed reasoning response should parse");

    assert_eq!(response.assistant_message, "Visible");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("gpt-5.5-thinking-extra-high")
    );
}

#[test]
fn preserves_reasoning_items_for_stateless_tool_continuation() {
    let response = parse_response(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        ThinkingLevel::ExtraHigh,
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_1",
                    "summary": [{ "type": "summary_text", "text": "Need the tool." }],
                    "encrypted_content": "encrypted-reasoning"
                },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            ]
        }),
        None,
    )
    .expect("reasoning response should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(
        response.provider_context,
        vec![ProviderConversationItem::OpenAiReasoning {
            id: "rs_1".to_owned(),
            summary: vec![OpenAiReasoningSummary {
                kind: "summary_text".to_owned(),
                text: "Need the tool.".to_owned(),
            }],
            encrypted_content: Some("encrypted-reasoning".to_owned()),
        }]
    );
}

#[test]
fn parses_usage_token_details() {
    let response = parse_response(
        "gpt-5.5-thinking-medium",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Done" }]
            }],
            "usage": {
                "input_tokens": 120,
                "output_tokens": 32,
                "total_tokens": 152,
                "input_tokens_details": { "cached_tokens": 40 },
                "output_tokens_details": { "reasoning_tokens": 12 }
            }
        }),
        None,
    )
    .expect("usage details should parse");

    assert_eq!(response.usage.input_tokens, 80);
    assert_eq!(response.usage.output_tokens, 20);
    assert_eq!(response.usage.total_tokens, 152);
    assert_eq!(response.usage.cached_input_tokens, 40);
    assert_eq!(response.usage.reasoning_tokens, 12);
}

#[test]
fn missing_total_tokens_falls_back_to_normalized_usage_sum() {
    let response = parse_response(
        "gpt-5.5-thinking-medium",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Done" }]
            }],
            "usage": {
                "input_tokens": 120,
                "output_tokens": 32,
                "input_tokens_details": { "cached_tokens": 40 },
                "output_tokens_details": { "reasoning_tokens": 12 }
            }
        }),
        None,
    )
    .expect("usage details should parse");

    assert_eq!(response.usage.input_tokens, 80);
    assert_eq!(response.usage.output_tokens, 20);
    assert_eq!(response.usage.cached_input_tokens, 40);
    assert_eq!(response.usage.reasoning_tokens, 12);
    assert_eq!(response.usage.total_tokens, 152);
}

fn openai_text_body(text: &str) -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text }]
        }]
    })
}

fn openai_function_call_body(arguments: &str) -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "function_call",
            "call_id": "call_1",
            "name": "memory_read",
            "arguments": arguments
        }]
    })
}
