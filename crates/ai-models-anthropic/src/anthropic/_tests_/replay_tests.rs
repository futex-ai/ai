//! Signed assistant content survives streamed parsing and stored-message replay.

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use serde_json::json;

use crate::{AnthropicModel, CLAUDE_OPUS_5, CLAUDE_SONNET_4_6};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn signed_anthropic_blocks_replay_in_full_original_order() {
    for blocks in [
        json!([
            {"type": "thinking", "thinking": "", "signature": "omitted-signature"},
            {"type": "text", "text": "  \n  "},
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
        let model = AnthropicModel::new(client.clone(), CLAUDE_OPUS_5, "key");
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::user("look up 7")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        request.controls.generation.effort = Some(Effort::Max);
        let response = model.complete(&request).await.unwrap();
        assert_eq!(
            response
                .parts
                .iter()
                .flat_map(|part| {
                    part.provider_state().unwrap().payload["blocks"]
                        .as_array()
                        .unwrap()
                        .clone()
                })
                .collect::<Vec<_>>(),
            blocks.as_array().unwrap().clone()
        );
        assert!(!response.text().contains("private thought"));
        let assistant = response.to_assistant_message();
        let stored = serde_json::from_value(serde_json::to_value(assistant).unwrap()).unwrap();
        request.messages.push(stored);
        request
            .messages
            .push(ConversationMessage::assistant("next message", Vec::new()));
        request
            .messages
            .push(ConversationMessage::tool("7", "lookup", "call-1"));
        AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key")
            .complete(&request)
            .await
            .unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                requests[1]
                    .body
                    .as_ref()
                    .unwrap()
                    .as_json_bytes()
                    .expect("JSON request body")
            )
            .expect("valid JSON request body")["messages"][1]["content"],
            blocks
        );
    }
}
