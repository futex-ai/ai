//! Legacy item fields survive one-way conversion to opaque state.

use serde_json::json;

use super::assert_preserved_legacy_item;

#[test]
fn minimax_assistant_context_round_trips_all_reasoning_fields() {
    assert_preserved_legacy_item(
        "minimax",
        json!({
            "type": "minimax_assistant",
            "reasoning_content": "private reasoning",
            "reasoning_details": [{
                "type": "reasoning.text",
                "id": "reasoning-text-1",
                "format": "MiniMax-response-v1",
                "index": 0,
                "text": "private reasoning"
            }]
        }),
    );
}

#[test]
fn minimax_assistant_context_omits_absent_reasoning_fields() {
    assert_preserved_legacy_item(
        "minimax",
        json!({
            "type": "minimax_assistant",
            "reasoning_details": [{}]
        }),
    );
}
