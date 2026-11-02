//! Legacy item fields survive one-way conversion to opaque state.

use serde_json::json;

use super::assert_preserved_legacy_item;

#[test]
fn deepseek_assistant_context_deserializes_empty_content_and_ordered_raw_calls() {
    assert_preserved_legacy_item(
        "deepseek",
        json!({
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
        }),
    );
}

#[test]
fn deepseek_assistant_context_round_trips_private_reasoning() {
    assert_preserved_legacy_item(
        "deepseek",
        json!({
            "type": "deep_seek_assistant_message",
            "content": "I will check.",
            "reasoning_content": "Need both calls.",
            "tool_calls": [{
                "id": "call_123",
                "name": "memory_read",
                "arguments": "{ \"path\": \"root\" }"
            }]
        }),
    );
}
