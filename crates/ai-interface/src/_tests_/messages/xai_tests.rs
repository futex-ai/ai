//! Legacy item fields survive one-way conversion to opaque state.

use serde_json::json;

use super::assert_preserved_legacy_item;

#[test]
fn xai_legacy_function_call_context_serializes_with_provider_tag() {
    assert_preserved_legacy_item(
        "xai",
        json!({
            "type": "xai_legacy_function_call",
            "tool_call_id": "xai_legacy_function_call:memory_read",
            "name": "memory_read",
            "arguments": "{\"path\":\"root\"}"
        }),
    );
}
