//! DeepSeek provider conversation-item serde tests.

use crate::{DeepSeekToolCallContext, ProviderConversationItem};
use serde_json::json;

#[test]
fn deepseek_assistant_context_deserializes_empty_content_and_ordered_raw_calls() {
    let item: ProviderConversationItem = serde_json::from_value(json!({
        "type": "deep_seek_assistant_message",
        "content": "",
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
        ProviderConversationItem::DeepSeekAssistantMessage {
            content: String::new(),
            reasoning_content: None,
            tool_calls: vec![
                DeepSeekToolCallContext {
                    id: "call_first".to_owned(),
                    name: "memory_read".to_owned(),
                    arguments: "{\n  \"path\": \"first\"\n}".to_owned(),
                },
                DeepSeekToolCallContext {
                    id: "call_second".to_owned(),
                    name: "memory_write".to_owned(),
                    arguments: "{\"path\":\"second\",\"value\":2}".to_owned(),
                },
            ],
        }
    );
}

#[test]
fn deepseek_assistant_context_round_trips_private_reasoning() {
    let item = ProviderConversationItem::DeepSeekAssistantMessage {
        content: "I will check.".to_owned(),
        reasoning_content: Some("Need both calls.".to_owned()),
        tool_calls: vec![DeepSeekToolCallContext {
            id: "call_123".to_owned(),
            name: "memory_read".to_owned(),
            arguments: "{ \"path\": \"root\" }".to_owned(),
        }],
    };
    let value = serde_json::to_value(&item).unwrap();

    assert_eq!(
        value,
        json!({
            "type": "deep_seek_assistant_message",
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
