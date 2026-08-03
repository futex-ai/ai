//! Kimi provider conversation-item serde tests.

use crate::{KimiToolCallContext, ProviderConversationItem};
use serde_json::json;

#[test]
fn kimi_assistant_context_deserializes_nullable_fields_and_ordered_raw_calls() {
    let item: ProviderConversationItem = serde_json::from_value(json!({
        "type": "kimi_assistant_message",
        "content": null,
        "reasoning_content": null,
        "tool_calls": [
            {
                "id": "call_first",
                "name": "memory_read",
                "arguments": "{\n  \"path\": \"first\"\n}"
            },
            {
                "id": "call_second",
                "name": "memory_write",
                "arguments": "{\"path\":\"second\",\"value\":2}"
            }
        ]
    }))
    .unwrap();

    assert_eq!(
        item,
        ProviderConversationItem::KimiAssistantMessage {
            content: None,
            reasoning_content: None,
            tool_calls: vec![
                KimiToolCallContext {
                    id: "call_first".to_owned(),
                    name: "memory_read".to_owned(),
                    arguments: "{\n  \"path\": \"first\"\n}".to_owned(),
                },
                KimiToolCallContext {
                    id: "call_second".to_owned(),
                    name: "memory_write".to_owned(),
                    arguments: "{\"path\":\"second\",\"value\":2}".to_owned(),
                },
            ],
        }
    );
}

#[test]
fn kimi_assistant_context_omits_absent_optional_fields() {
    let item = ProviderConversationItem::KimiAssistantMessage {
        content: None,
        reasoning_content: None,
        tool_calls: Vec::new(),
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "kimi_assistant_message"
        })
    );
}

#[test]
fn kimi_assistant_context_round_trips_raw_provider_fields() {
    let item = ProviderConversationItem::KimiAssistantMessage {
        content: Some("I will check.".to_owned()),
        reasoning_content: Some("Need both calls.".to_owned()),
        tool_calls: vec![KimiToolCallContext {
            id: "call_123".to_owned(),
            name: "memory_read".to_owned(),
            arguments: "{ \"path\": \"root\" }".to_owned(),
        }],
    };
    let value = serde_json::to_value(&item).unwrap();

    assert_eq!(
        value,
        json!({
            "type": "kimi_assistant_message",
            "content": "I will check.",
            "reasoning_content": "Need both calls.",
            "tool_calls": [{
                "id": "call_123",
                "name": "memory_read",
                "arguments": "{ \"path\": \"root\" }"
            }]
        })
    );
    assert_eq!(
        serde_json::from_value::<ProviderConversationItem>(value).unwrap(),
        item
    );
}
