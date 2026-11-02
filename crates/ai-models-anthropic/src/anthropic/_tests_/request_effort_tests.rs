//! Request effort overrides the default for one call on a shared model.

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use serde_json::json;

use crate::{AnthropicModel, CLAUDE_OPUS_4_7};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn request_effort_overrides_default_for_each_call() {
    let body = json!({
        "content": [{"type": "text", "text": "Done"}],
        "stop_reason": "end_turn"
    });
    let (client, requests) = client_for_buffered_bodies(vec![body.clone(), body]);
    let model = AnthropicModel::new(client, CLAUDE_OPUS_4_7, "key");
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.effort = Some(Effort::Max);

    let explicit = model.complete(&request).await.unwrap();
    assert_eq!(explicit.effort().map(Effort::as_str), Some("max"));

    request.controls.generation.effort = None;
    let default = model.complete(&request).await.unwrap();
    assert_eq!(default.effort().map(Effort::as_str), Some("disabled"));

    let requests = requests.lock().unwrap();
    assert_eq!(
        requests[0].body.as_ref().unwrap()["output_config"]["effort"],
        "max"
    );
    assert!(requests[1].body.as_ref().unwrap().get("thinking").is_none());
}
