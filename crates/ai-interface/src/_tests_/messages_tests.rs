use crate::{MiniMaxReasoningDetail, OpenAiReasoningSummary, ProviderConversationItem};
use serde_json::json;

#[test]
fn openai_message_context_serializes_with_provider_tag() {
    let item = ProviderConversationItem::OpenAiMessage {
        phase: Some("commentary".to_owned()),
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
fn openai_message_context_without_phase_serializes_with_provider_tag() {
    let item = ProviderConversationItem::OpenAiMessage { phase: None };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "openai_message"
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

#[test]
fn xai_legacy_function_call_context_serializes_with_provider_tag() {
    let item = ProviderConversationItem::XaiLegacyFunctionCall {
        tool_call_id: "xai_legacy_function_call:memory_read".to_owned(),
        name: "memory_read".to_owned(),
        arguments: "{\"path\":\"root\"}".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "xai_legacy_function_call",
            "tool_call_id": "xai_legacy_function_call:memory_read",
            "name": "memory_read",
            "arguments": "{\"path\":\"root\"}"
        })
    );
}

#[test]
fn minimax_assistant_context_round_trips_all_reasoning_fields() {
    let item = ProviderConversationItem::MiniMaxAssistant {
        reasoning_content: Some("private reasoning".to_owned()),
        reasoning_details: vec![MiniMaxReasoningDetail {
            kind: Some("reasoning.text".to_owned()),
            id: Some("reasoning-text-1".to_owned()),
            format: Some("MiniMax-response-v1".to_owned()),
            index: Some(0),
            text: Some("private reasoning".to_owned()),
        }],
    };
    let serialized = json!({
        "type": "minimax_assistant",
        "reasoning_content": "private reasoning",
        "reasoning_details": [{
            "type": "reasoning.text",
            "id": "reasoning-text-1",
            "format": "MiniMax-response-v1",
            "index": 0,
            "text": "private reasoning"
        }]
    });

    assert_eq!(serde_json::to_value(&item).unwrap(), serialized);
    assert_eq!(
        serde_json::from_value::<ProviderConversationItem>(serialized).unwrap(),
        item
    );
}

#[test]
fn minimax_assistant_context_omits_absent_reasoning_fields() {
    let item = ProviderConversationItem::MiniMaxAssistant {
        reasoning_content: None,
        reasoning_details: vec![MiniMaxReasoningDetail {
            kind: None,
            id: None,
            format: None,
            index: None,
            text: None,
        }],
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "minimax_assistant",
            "reasoning_details": [{}]
        })
    );
}
