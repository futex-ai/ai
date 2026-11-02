//! Signed assistant content survives streamed parsing and stored-message replay.

use ai_interface::{ConversationMessage, Model, ModelRequest};
use serde_json::json;

use crate::{AnthropicModel, CLAUDE_OPUS_5_THINKING_MAX};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn signed_anthropic_blocks_replay_in_full_original_order() {
    for blocks in [
        json!([
            {"type": "thinking", "thinking": "", "signature": "omitted-signature"},
            {"type": "text", "text": "  before  "},
            {"type": "thinking", "thinking": "private thought", "signature": "signature"},
            {"type": "redacted_thinking", "data": "encrypted-data"},
            {"type": "tool_use", "id": "call-1", "name": "lookup", "input": {"id": 7}},
            {"type": "text", "text": "after"}
        ]),
        json!([
            {"type": "redacted_thinking", "data": "redacted-only"},
            {"type": "tool_use", "id": "call-1", "name": "lookup", "input": {"id": 7}}
        ]),
    ] {
        let (client, requests) = client_for_buffered_bodies(vec![
            json!({"content": blocks, "stop_reason": "tool_use"}),
            json!({"content": [{"type": "text", "text": "Done"}], "stop_reason": "end_turn"}),
        ]);
        let model = AnthropicModel::new(client, CLAUDE_OPUS_5_THINKING_MAX, "key");
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::user("look up 7")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        let response = model.complete(&request).await.unwrap();
        assert_eq!(
            serde_json::to_value(&response.provider_context).unwrap(),
            json!([
                {"type": "anthropic_assistant_message", "blocks": blocks}
            ])
        );
        assert!(!response.assistant_message.contains("private thought"));
        let assistant = ConversationMessage::assistant_with_provider_context(
            "normalized fallback",
            Vec::new(),
            response.provider_context,
        );
        let stored = serde_json::from_value(serde_json::to_value(assistant).unwrap()).unwrap();
        request.messages.push(stored);
        request
            .messages
            .push(ConversationMessage::assistant("next message", Vec::new()));
        request
            .messages
            .push(ConversationMessage::tool("7", "lookup", "call-1"));
        model.complete(&request).await.unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests[1].body.as_ref().unwrap()["messages"][1]["content"],
            blocks
        );
    }
}
