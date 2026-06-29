//! Reasoning output item response tests.

use ai_interface::{FinishReason, OpenAiReasoningSummary, ProviderConversationItem};
use serde_json::json;

use super::{ThinkingLevel, parse_response};

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
