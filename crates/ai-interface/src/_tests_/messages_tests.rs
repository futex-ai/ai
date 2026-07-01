use crate::{OpenAiReasoningSummary, ProviderConversationItem};
use serde_json::json;

#[test]
fn openai_message_context_serializes_with_provider_tag() {
    let item = ProviderConversationItem::OpenAiMessage {
        phase: "commentary".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "openai_message",
            "phase": "commentary"
        })
    );
}

#[test]
fn openai_reasoning_context_serializes_with_provider_tag() {
    let item = ProviderConversationItem::OpenAiReasoning {
        id: "rs_123".to_owned(),
        summary: vec![OpenAiReasoningSummary {
            kind: "summary_text".to_owned(),
            text: "Used the retained plan.".to_owned(),
        }],
        encrypted_content: Some("encrypted".to_owned()),
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "openai_reasoning",
            "id": "rs_123",
            "summary": [{
                "type": "summary_text",
                "text": "Used the retained plan."
            }],
            "encrypted_content": "encrypted"
        })
    );
}

#[test]
fn openai_function_call_context_serializes_with_provider_tag() {
    let item = ProviderConversationItem::OpenAiFunctionCall {
        id: Some("fc_123".to_owned()),
        call_id: "call_123".to_owned(),
        name: "memory_read".to_owned(),
        arguments: "{\n  \"path\": \"root\"\n}".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "openai_function_call",
            "id": "fc_123",
            "call_id": "call_123",
            "name": "memory_read",
            "arguments": "{\n  \"path\": \"root\"\n}"
        })
    );
}
