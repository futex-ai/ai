//! Thinking models reject strict tool choice and preserve automatic fallback.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, Effort, Model, ModelControl, ModelError, ModelRequest, ModelToolChoice,
    ToolDefinition,
};
use json_http::TransportBackedJsonHttpClient;
use serde_json::json;
use unimock::Unimock;

use crate::{AnthropicModel, CLAUDE_OPUS_5};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn thinking_tool_choice_rejects_strict_modes_before_transport() {
    for choice in [
        ModelToolChoice::Required,
        ModelToolChoice::Function("lookup".to_owned()),
    ] {
        let transport = Unimock::new(());
        let model = AnthropicModel::new(
            Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
            CLAUDE_OPUS_5,
            "key",
        );
        let result = model.complete(&request(choice)).await;
        assert!(
            matches!(
                result,
                Err(ModelError::UnsupportedControl {
                    control: ModelControl::ToolChoice,
                    ..
                })
            ),
            "{result:?}"
        );
    }
}

#[tokio::test]
async fn thinking_tool_choice_required_or_auto_keeps_tools_and_sends_auto() {
    let (client, requests) = client_for_buffered_bodies(vec![json!({
        "content": [{"type": "text", "text": "Done"}], "stop_reason": "end_turn"
    })]);
    let model = AnthropicModel::new(client, CLAUDE_OPUS_5, "key");
    model
        .complete(&request(ModelToolChoice::RequiredOrAuto))
        .await
        .unwrap();
    let requests = requests.lock().unwrap();
    let body = requests[0].body.as_ref().unwrap();
    assert_eq!(body["tool_choice"], json!({"type": "auto"}));
    assert_eq!(body["tools"][0]["name"], "lookup");
    assert_eq!(body["thinking"]["type"], "adaptive");
}

fn request(choice: ModelToolChoice) -> ModelRequest {
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("Call lookup")],
        tools: vec![ToolDefinition {
            name: "lookup".to_owned(),
            description: "Look up a value.".to_owned(),
            input_schema: json!({"type": "object"}),
            activity_verb: None,
        }],
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.tool_choice = Some(choice);
    request.controls.generation.effort = Some(Effort::Max);
    request
}
