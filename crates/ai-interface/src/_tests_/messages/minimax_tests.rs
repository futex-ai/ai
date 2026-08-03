//! MiniMax provider conversation-item serde tests.

use crate::{MiniMaxReasoningDetail, ProviderConversationItem};
use serde_json::json;

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
