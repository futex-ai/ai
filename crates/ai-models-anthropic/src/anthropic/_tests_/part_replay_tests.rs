//! Portable part replay, invalid own state, and role validation regressions.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, Model, ModelError, ModelRequest,
    ProviderState, ToolCall,
};
use json_http::{JsonHttpClient, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::Unimock;

use crate::{AnthropicModel, CLAUDE_SONNET_4_6};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn unsigned_multiblock_continuation_keeps_normalized_v1_request() {
    let (client, requests) = client_for_buffered_bodies(vec![
        json!({"stop_reason": "tool_use", "content": [
            {"type": "text", "text": "  "},
            {"type": "text", "text": "First"},
            {"type": "tool_use", "id": "call", "name": "lookup", "input": {"id": 7}},
            {"type": "text", "text": "Second"}
        ]}),
        json!({"stop_reason": "end_turn", "content": [{"type": "text", "text": "Done"}]}),
    ]);
    let model = AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key");
    let mut request = request(vec![ConversationMessage::user("find 7")]);
    let response = model.complete(&request).await.unwrap();
    assert_eq!(response.text(), "First\nSecond");
    request.messages.push(response.to_assistant_message());
    request
        .messages
        .push(ConversationMessage::tool("7", "lookup", "call"));
    model.complete(&request).await.unwrap();
    assert_eq!(
        requests.lock().unwrap()[1].body.as_ref().unwrap()["messages"][1]["content"],
        json!([
            {"type": "text", "text": "First\nSecond"},
            {"type": "tool_use", "id": "call", "name": "lookup", "input": {"id": 7}}
        ])
    );
}

#[tokio::test]
async fn foreign_state_replays_portable_parts_and_skips_foreign_or_absent_reasoning() {
    let state: ProviderState = serde_json::from_value(json!({
        "provider": "other", "model": "other-model", "version": 42, "payload": {"invalid": true}
    }))
    .unwrap();
    let (client, requests) = client_for_buffered_bodies(vec![json!({
        "stop_reason": "end_turn", "content": [{"type": "text", "text": "Done"}]
    })]);
    let model = AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key");
    let message = ConversationMessage::new(
        ConversationRole::Assistant,
        vec![
            ConversationPart::reasoning("own reasoning").with_provider_state(
                serde_json::from_value(json!({
                    "provider": "anthropic", "model": "another-model", "version": 1,
                    "payload": {"scope": "part", "blocks": [
                        {"type": "thinking", "thinking": "own reasoning", "signature": "signed"}
                    ]}
                }))
                .unwrap(),
            ),
            ConversationPart::reasoning("private").with_provider_state(state.clone()),
            ConversationPart::reasoning("also private"),
            ConversationPart::text("Visible").with_provider_state(state.clone()),
            ConversationPart::from_tool_call(ToolCall {
                id: "call".to_owned(),
                name: "lookup".to_owned(),
                input: json!({"id": 7}),
                operation_id: Some("operation".to_owned()),
            })
            .with_provider_state(state),
        ],
    );
    model.complete(&request(vec![message])).await.unwrap();
    assert_eq!(
        requests.lock().unwrap()[0].body.as_ref().unwrap()["messages"],
        json!([{"role": "assistant", "content": [
            {"type": "thinking", "thinking": "own reasoning", "signature": "signed"},
            {"type": "text", "text": "Visible"},
            {"type": "tool_use", "id": "call", "name": "lookup", "input": {"id": 7}}
        ]}])
    );
}

#[tokio::test]
async fn invalid_own_state_and_roles_fail_before_transport() {
    let client: Arc<dyn JsonHttpClient> = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(
        Unimock::new(()),
    )));
    let model = AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key");
    let cases = [
        (
            ConversationMessage::new(
                ConversationRole::Assistant,
                vec![
                    legacy_message_part(),
                    ConversationPart::reasoning("").with_provider_state(
                        serde_json::from_value(json!({
                            "provider": "anthropic", "model": "any", "version": 2, "payload": {}
                        }))
                        .unwrap(),
                    ),
                ],
            ),
            "[ai_models_anthropic/replay] unsupported provider state version 2",
        ),
        (
            ConversationMessage::new(
                ConversationRole::Assistant,
                vec![
                    legacy_message_part(),
                    ConversationPart::text("visible").with_provider_state(
                        serde_json::from_value(json!({
                            "provider": "anthropic", "model": "", "version": 1, "payload": {}
                        }))
                        .unwrap(),
                    ),
                ],
            ),
            "[ai_models_anthropic/replay] invalid provider state payload",
        ),
        (
            ConversationMessage::new(
                ConversationRole::Assistant,
                vec![
                    ConversationPart::reasoning("").with_provider_state(
                        serde_json::from_value(json!({
                            "provider": "anthropic", "model": "", "version": 1,
                            "payload": {"blocks": [{"type": "private-replay-secret"}]}
                        }))
                        .unwrap(),
                    ),
                ],
            ),
            "[ai_models_anthropic/replay] invalid provider state payload",
        ),
        (
            ConversationMessage::new(
                ConversationRole::User,
                vec![ConversationPart::reasoning("no")],
            ),
            "conversation parts do not match the message role",
        ),
        (
            ConversationMessage::new(ConversationRole::Tool, Vec::new()),
            "conversation parts do not match the message role",
        ),
        (
            ConversationMessage::new(
                ConversationRole::Assistant,
                vec![ConversationPart::image("image/png", "data")],
            ),
            "conversation parts do not match the message role",
        ),
    ];
    for (message, expected_message) in cases {
        let error = model.complete(&request(vec![message])).await.unwrap_err();
        let ModelError::Provider {
            provider,
            model_id,
            message,
        } = error
        else {
            panic!("expected invalid request provider error, got {error:?}");
        };
        assert_eq!(provider, "anthropic");
        assert_eq!(model_id, "claude-sonnet-4-6");
        assert_eq!(message, expected_message);
    }
}

fn legacy_message_part() -> ConversationPart {
    ConversationPart::reasoning("").with_provider_state(
        serde_json::from_value(json!({
            "provider": "anthropic", "model": "", "version": 1,
            "payload": {"blocks": [{"type": "thinking", "thinking": "", "signature": "valid"}]}
        }))
        .unwrap(),
    )
}

fn request(messages: Vec<ConversationMessage>) -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages,
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
