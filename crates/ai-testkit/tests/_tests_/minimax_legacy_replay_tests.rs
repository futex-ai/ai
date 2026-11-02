//! Version one MiniMax reasoning replays only its first whole-message snapshot.

use ai_interface::ConversationMessage;
use ai_testkit::provider::Provider;
use serde_json::{Value, json};

use super::{converter, legacy_request_support::request_body};

#[tokio::test]
async fn converted_minimax_replays_only_first_legacy_reasoning_item() {
    let message = converter().read(legacy_message()).unwrap();
    assert_first_snapshot(message).await;
}

#[tokio::test]
async fn serde_minimax_replays_only_first_legacy_reasoning_item() {
    let message = serde_json::from_value(legacy_message()).unwrap();
    assert_first_snapshot(message).await;
}

async fn assert_first_snapshot(message: ConversationMessage) {
    let stored = serde_json::to_value(message).unwrap();
    let message = serde_json::from_value(stored).unwrap();
    let body = request_body(Provider::Minimax, message).await;
    assert_eq!(
        body["messages"],
        json!([{
            "role": "assistant", "content": "visible", "reasoning_content": "first",
            "reasoning_details": [{
                "type": "reasoning.text", "id": "d1", "format": "minimax",
                "index": 0, "text": "first detail"
            }]
        }])
    );
}

fn legacy_message() -> Value {
    json!({
        "role": "assistant", "content": "visible",
        "provider_context": [
            {
                "type": "minimax_assistant", "reasoning_content": "first",
                "reasoning_details": [{
                    "type": "reasoning.text", "id": "d1", "format": "minimax",
                    "index": 0, "text": "first detail"
                }]
            },
            {
                "type": "minimax_assistant", "reasoning_content": "second",
                "reasoning_details": [{
                    "type": "reasoning.text", "id": "d2", "format": "minimax",
                    "index": 0, "text": "second detail"
                }]
            }
        ]
    })
}
