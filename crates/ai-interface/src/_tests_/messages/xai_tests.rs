//! xAI provider conversation-item serde tests.

use crate::ProviderConversationItem;
use serde_json::json;

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
