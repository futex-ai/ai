//! Request effort changes on each call without rebuilding the OpenAI model.

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use serde_json::json;

use crate::{GPT_5_6_SOL, OpenAiModel};

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn request_effort_overrides_default_and_caps_output_for_each_call() {
    let body = json!({"status": "completed", "output": [{
        "type": "message", "role": "assistant",
        "content": [{"type": "output_text", "text": "Done"}]
    }]});
    let (client, requests) = client_for_buffered_bodies(vec![body.clone(), body]);
    let model = OpenAiModel::new(client, GPT_5_6_SOL, "key");
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.effort = Some(Effort::Max);
    request.controls.generation.max_output_tokens = Some(200_000);

    let explicit = model.complete(&request).await.unwrap();
    assert_eq!(explicit.effort().map(Effort::as_str), Some("max"));

    request.controls.generation.effort = None;
    let default = model.complete(&request).await.unwrap();
    assert_eq!(default.effort().map(Effort::as_str), Some("medium"));

    let requests = requests.lock().unwrap();
    assert_eq!(
        requests[0].body.as_ref().unwrap()["reasoning"]["effort"],
        "max"
    );
    assert_eq!(
        requests[1].body.as_ref().unwrap()["reasoning"]["effort"],
        "medium"
    );
    assert_eq!(
        requests[0].body.as_ref().unwrap()["max_output_tokens"],
        128_000
    );
}
