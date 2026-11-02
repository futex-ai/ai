//! Stored version one Anthropic blocks keep their exact continuation request.

use ai_interface::{ConversationRole, Model, ModelRequest, read_stored_message};
use serde_json::json;

use crate::{AnthropicLegacyConverter, AnthropicModel, CLAUDE_SONNET_4_6};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn converts_full_v1_block_order_and_preserves_portable_operation_id() {
    let blocks = json!([
        {"type": "text", "text": "  "},
        {"type": "thinking", "thinking": "", "signature": "signed"},
        {"type": "text", "text": " before "},
        {"type": "redacted_thinking", "data": "redacted"},
        {"type": "tool_use", "id": "call", "name": "lookup", "input": {"id": 7}},
        {"type": "text", "text": "after"}
    ]);
    let message = read_stored_message(json!({
        "role": "assistant", "content": " before \nafter", "name": "participant",
        "tool_calls": [{"id": "call", "name": "lookup", "input": {"id": 7}, "operation_id": "operation"}],
        "provider_context": [{"type": "anthropic_assistant_message", "blocks": blocks}]
    }), &[&AnthropicLegacyConverter]).unwrap();
    assert_eq!(message.role, ConversationRole::Assistant);
    assert_eq!(message.name.as_deref(), Some("participant"));
    assert_eq!(message.text(), " before \nafter");
    assert_eq!(
        message.tool_calls()[0].operation_id.as_deref(),
        Some("operation")
    );
    assert_eq!(
        message
            .parts
            .iter()
            .map(|part| part.provider_state().unwrap().model.as_str())
            .collect::<Vec<_>>(),
        vec!["", "", "", "", "", ""]
    );
    let (client, requests) = client_for_buffered_bodies(vec![json!({
        "stop_reason": "end_turn", "content": [{"type": "text", "text": "Done"}]
    })]);
    AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key")
        .complete(&ModelRequest {
            system_prompt: String::new(),
            messages: vec![message],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .unwrap();
    assert_eq!(
        requests.lock().unwrap()[0].body.as_ref().unwrap()["messages"],
        json!([{"role": "assistant", "content": blocks}])
    );
}

#[tokio::test]
async fn legacy_raw_blocks_override_portable_text_and_keep_empty_message_boundary() {
    for blocks in [
        json!([]),
        json!([{"type": "text", "text": "original raw text"}]),
        json!([{"type": "thinking", "thinking": "", "signature": "signed"}]),
    ] {
        let message = read_stored_message(
            json!({
                "role": "assistant", "content": "portable fallback",
                "provider_context": [{"type": "anthropic_assistant_message", "blocks": blocks}]
            }),
            &[&AnthropicLegacyConverter],
        )
        .unwrap();
        assert_eq!(message.text(), "portable fallback");
        let (client, requests) = client_for_buffered_bodies(vec![json!({
            "stop_reason": "end_turn", "content": [{"type": "text", "text": "Done"}]
        })]);
        AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key")
            .complete(&ModelRequest {
                system_prompt: String::new(),
                messages: vec![message],
                tools: Vec::new(),
                response_schema: None,
                controls: Default::default(),
            })
            .await
            .unwrap();
        assert_eq!(
            requests.lock().unwrap()[0].body.as_ref().unwrap()["messages"],
            json!([{"role": "assistant", "content": blocks}])
        );
    }
}
