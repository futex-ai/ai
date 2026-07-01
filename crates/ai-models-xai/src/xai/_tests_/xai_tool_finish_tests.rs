//! Tests for xAI tool-call terminal finish handling.

use ai_interface::{FinishReason, ProviderConversationItem};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::response::parse_response;

#[test]
fn truncated_tool_call_response_does_not_parse_partial_arguments() {
    let response = parse_response(
        "grok-4",
        "grok-4",
        ThinkingLevel::Disabled,
        "test_scope",
        json!({
            "choices": [{
                "finish_reason": "length",
                "message": {
                    "content": "",
                    "tool_calls": [{
                        "id": "call_1",
                        "function": {
                            "name": "memory_read",
                            "arguments": "{\"path\":"
                        }
                    }]
                }
            }]
        }),
        None,
    )
    .expect("truncated xAI tool-call response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert!(response.tool_calls.is_empty());
}

#[test]
fn legacy_function_call_response_parses_tool_call() {
    let response = parse_response(
        "grok-4",
        "grok-4",
        ThinkingLevel::Disabled,
        "test_scope",
        json!({
            "choices": [{
                "finish_reason": "function_call",
                "message": {
                    "content": "",
                    "function_call": {
                        "name": "memory_read",
                        "arguments": "{\"path\":\"root\"}"
                    }
                }
            }]
        }),
        None,
    )
    .expect("legacy xAI function-call response should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert!(
        response.tool_calls[0]
            .id
            .starts_with("xai_legacy_function_call:")
    );
    assert_eq!(
        response.tool_calls[0].operation_id.as_deref(),
        Some(response.tool_calls[0].id.as_str())
    );
    assert_eq!(response.tool_calls[0].name, "memory_read");
    assert_eq!(response.tool_calls[0].input, json!({"path": "root"}));
    assert_eq!(
        response.provider_context,
        vec![ProviderConversationItem::XaiLegacyFunctionCall {
            tool_call_id: response.tool_calls[0].id.clone(),
            name: "memory_read".to_owned(),
            arguments: "{\"path\":\"root\"}".to_owned(),
        }]
    );
}
