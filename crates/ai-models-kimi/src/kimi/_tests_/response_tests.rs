//! Kimi normalized response tests.

use ai_interface::{FinishReason, ModelError, ProviderConversationItem};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::{KIMI_K3, KIMI_K3_THINKING_HIGH};

use super::response::parse_response;

#[test]
fn nullable_content_normalizes_empty_and_reasoning_remains_replay_only() {
    let response = parse(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {
                "content": null,
                "reasoning_content": "Never expose this."
            }
        }]
    }))
    .expect("nullable Kimi content should parse");

    assert_eq!(response.assistant_message, "");
    assert!(!response.assistant_message.contains("Never expose"));
    assert_eq!(
        response.provider_context,
        vec![ProviderConversationItem::KimiAssistantMessage {
            content: None,
            reasoning_content: Some("Never expose this.".to_owned()),
            tool_calls: Vec::new(),
        }]
    );
}

#[test]
fn rejects_missing_choices_and_malformed_payloads() {
    assert!(matches!(
        parse(json!({"choices": []})),
        Err(ModelError::Provider { .. })
    ));
    assert!(matches!(
        parse(json!({"choices": [{"finish_reason": "stop"}]})),
        Err(ModelError::Provider { .. })
    ));
    assert!(matches!(
        parse(json!({"choices": "not-an-array"})),
        Err(ModelError::Provider { .. })
    ));
}

#[test]
fn maps_all_finish_reasons_and_selected_model_metadata() {
    let cases = [
        (Some("stop"), FinishReason::Stop),
        (Some("tool_calls"), FinishReason::ToolCalls),
        (Some("length"), FinishReason::Truncated),
        (Some("content_filter"), FinishReason::Filtered),
        (
            Some("custom_reason"),
            FinishReason::Other("custom_reason".to_owned()),
        ),
        (None, FinishReason::Other("missing".to_owned())),
    ];

    for (raw, expected) in cases {
        let response = parse(response_body(raw)).expect("finish reason should parse");

        assert_eq!(response.finish_reason, expected);
        assert_eq!(response.provider, "kimi");
        assert_eq!(response.model_id, KIMI_K3);
        assert_eq!(
            response.catalog_model_id.as_deref(),
            Some(KIMI_K3_THINKING_HIGH)
        );
        assert_eq!(response.thinking_level.as_deref(), Some("high"));
    }
}

fn parse(body: Value) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response(
        KIMI_K3_THINKING_HIGH,
        KIMI_K3,
        ThinkingLevel::High,
        body,
        None,
    )
}

fn response_body(finish_reason: Option<&str>) -> Value {
    let tool_calls = if matches!(finish_reason, Some("tool_calls")) {
        json!([{
            "id": "call_1",
            "type": "function",
            "function": {
                "name": "memory_read",
                "arguments": "{\"path\":\"root\"}"
            }
        }])
    } else {
        Value::Null
    };
    json!({
        "choices": [{
            "finish_reason": finish_reason,
            "message": {
                "content": "Done",
                "reasoning_content": "hidden",
                "tool_calls": tool_calls
            }
        }]
    })
}
