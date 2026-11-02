//! Legacy item fields survive one-way conversion to opaque state.

use serde_json::json;

use super::assert_preserved_legacy_item;

#[test]
fn openai_message_context_serializes_with_provider_tag() {
    assert_preserved_legacy_item(
        "openai",
        json!({
            "type": "openai_message",
            "phase": "commentary"
        }),
    );
}

#[test]
fn openai_message_context_without_phase_serializes_with_provider_tag() {
    assert_preserved_legacy_item(
        "openai",
        json!({
            "type": "openai_message"
        }),
    );
}

#[test]
fn openai_reasoning_context_serializes_with_provider_tag() {
    assert_preserved_legacy_item(
        "openai",
        json!({
            "type": "openai_reasoning",
            "id": "rs_123",
            "summary": [{
                "type": "summary_text",
                "text": "Used the retained plan."
            }],
            "encrypted_content": "encrypted"
        }),
    );
}

#[test]
fn openai_function_call_context_serializes_with_provider_tag() {
    assert_preserved_legacy_item(
        "openai",
        json!({
            "type": "openai_function_call",
            "id": "fc_123",
            "call_id": "call_123",
            "name": "memory_read",
            "arguments": "{\n  \"path\": \"root\"\n}"
        }),
    );
}
