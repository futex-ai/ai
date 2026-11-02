//! Shared legacy conversion assertions for original stored item fixtures.

use serde_json::{Value, json};

use crate::{ConversationMessage, ProviderId, ProviderState, read_stored_message};

pub(super) fn assert_preserved_legacy_item(provider: &str, item: Value) {
    let call_id = item
        .get("call_id")
        .or_else(|| item.get("tool_call_id"))
        .and_then(Value::as_str)
        .unwrap_or("call_123");
    let message = read_stored_message(json!({
        "role": "assistant", "content": "visible",
        "tool_calls": [{"id": call_id, "name": "memory_read", "input": {"path": "root"}, "operation_id": null}],
        "provider_context": [item]
    }), &[]).unwrap();
    let mut payload = item.as_object().unwrap().clone();
    payload.remove("type");
    assert_eq!(
        message
            .parts
            .iter()
            .filter_map(|part| part.provider_state())
            .cloned()
            .collect::<Vec<_>>(),
        vec![ProviderState {
            provider: ProviderId::from(provider.to_owned()),
            model: String::new(),
            version: 1,
            payload,
        }]
    );
    let value = serde_json::to_value(&message).unwrap();
    assert_eq!(
        serde_json::from_value::<ConversationMessage>(value).unwrap(),
        message
    );
}
