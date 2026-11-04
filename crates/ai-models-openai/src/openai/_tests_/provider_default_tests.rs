//! Provider-default reasoning and the explicit Responses off control.

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use json_http::JsonHttpBody;
use serde_json::{Value, json};

use crate::{GPT_5_4_MINI, GPT_5_4_NANO, OpenAiModel};

use super::stream_support::{client_for_buffered_bodies, custom_spec};

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn response() -> serde_json::Value {
    json!({"status": "completed", "output": [{
        "type": "message", "role": "assistant",
        "content": [{"type": "output_text", "text": "Done"}]
    }]})
}

#[tokio::test]
async fn provider_default_omits_reasoning_and_reports_no_effort() {
    let (http, requests) = client_for_buffered_bodies(vec![response()]);
    let model = OpenAiModel::new(http, custom_spec("gpt-custom"), "key");
    let mut request = request();
    request.controls.generation.temperature = Some(0.4);
    request.controls.generation.top_p = Some(0.8);
    let response = model.complete(&request).await.unwrap();
    assert_eq!(response.effort(), None);
    let requests = requests.lock().unwrap();
    let body = request_body(requests[0].body.as_ref().unwrap());
    assert_eq!(body.get("reasoning"), None);
    assert_eq!(body.get("temperature"), None);
    assert_eq!(body.get("top_p"), None);
    assert_eq!(body.get("include"), None);
}

#[tokio::test]
async fn explicit_off_sends_none_for_mini_and_nano() {
    for spec in [GPT_5_4_MINI, GPT_5_4_NANO] {
        let (http, requests) = client_for_buffered_bodies(vec![response()]);
        let model = OpenAiModel::new(http, spec, "key");
        let mut request = request();
        request.controls.generation.effort = Some(Effort::Disabled);
        let response = model.complete(&request).await.unwrap();
        let requests = requests.lock().unwrap();
        let body = request_body(requests[0].body.as_ref().unwrap());
        assert_eq!(body["reasoning"], json!({"effort": "none"}));
        assert_eq!(response.effort(), Some(Effort::Disabled));
    }
}

fn request_body(body: &JsonHttpBody) -> Value {
    serde_json::from_slice(body.as_json_bytes().expect("JSON request body"))
        .expect("valid JSON request body")
}
