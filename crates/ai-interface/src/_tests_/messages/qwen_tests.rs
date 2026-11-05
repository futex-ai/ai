//! Legacy item fields survive one-way conversion to opaque state.

use serde_json::json;

use super::assert_preserved_legacy_item;

#[test]
fn qwen_assistant_context_deserializes_nullable_fields_and_ordered_raw_calls() {
    assert_preserved_legacy_item(
        "qwen",
        json!({
            "type": "qwen_assistant_message",
            "content": null,
            "reasoning_content": "Use both sources.",
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
        }),
    );
}

#[test]
fn qwen_assistant_context_omits_absent_optional_fields() {
    assert_preserved_legacy_item(
        "qwen",
        json!({
            "type": "qwen_assistant_message"
        }),
    );
}

#[test]
fn qwen_assistant_context_round_trips_raw_provider_fields() {
    assert_preserved_legacy_item(
        "qwen",
        json!({
            "type": "qwen_assistant_message",
            "content": "I will check.",
            "reasoning_content": "Need the source.",
            "tool_calls": [{
                "id": "call_123",
                "name": "memory_read",
                "arguments": "{ \"path\": \"root\" }"
            }]
        }),
    );
}
