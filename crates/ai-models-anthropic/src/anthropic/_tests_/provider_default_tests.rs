//! Provider-default reasoning and the explicit off switch at transport.

use ai_interface::{
    ConversationMessage, Effort, ErrorKind, Model, ModelControl, ModelRequest, ModelSpec,
    ModelToolChoice,
};
use ai_models_core::resolve_controls;
use json_http::JsonHttpBody;
use serde_json::{Value, json};

use crate::{
    AnthropicModel, CLAUDE_FABLE_5, CLAUDE_HAIKU_4_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_5,
    CLAUDE_SONNET_4_6, CLAUDE_SONNET_5, CUSTOM_DEFAULTS, PROVIDER_ID,
};

use super::stream_support::client_for_buffered_bodies;

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
    json!({"content": [{"type": "text", "text": "Done"}], "stop_reason": "end_turn"})
}

#[tokio::test]
async fn provider_default_omits_reasoning_and_reports_no_effort() {
    let (http, requests) = client_for_buffered_bodies(vec![response()]);
    let spec = ModelSpec::custom(PROVIDER_ID, "claude-custom", &CUSTOM_DEFAULTS);
    let model = AnthropicModel::new(http, spec, "key");
    let mut request = request();
    request.controls.generation.temperature = Some(0.4);
    request.controls.generation.top_p = Some(0.8);
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);

    let response = model.complete(&request).await.unwrap();
    assert_eq!(response.effort(), None);
    let requests = requests.lock().unwrap();
    let body = request_body(requests[0].body.as_ref().unwrap());
    assert_eq!(body.get("thinking"), None);
    assert_eq!(body.get("output_config"), None);
    assert_eq!(body.get("temperature"), None);
    assert_eq!(body.get("top_p"), None);
    assert_eq!(body["tool_choice"], json!({"type": "auto"}));
}

#[tokio::test]
async fn explicit_off_sends_disabled_for_every_supported_claude() {
    for spec in [
        CLAUDE_SONNET_5,
        CLAUDE_OPUS_5,
        CLAUDE_SONNET_4_6,
        CLAUDE_OPUS_4_7,
        CLAUDE_HAIKU_4_5,
    ] {
        let id = spec.text_id();
        let (http, requests) = client_for_buffered_bodies(vec![response()]);
        let model = AnthropicModel::new(http, spec, "key");
        let mut request = request();
        request.controls.generation.effort = Some(Effort::Disabled);
        request.controls.generation.temperature = Some(0.4);
        request.controls.generation.tool_choice = Some(ModelToolChoice::Required);
        let response = model.complete(&request).await;
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{id}: {response:?}");
        let body = request_body(requests[0].body.as_ref().unwrap());
        assert_eq!(body["thinking"], json!({"type": "disabled"}), "{id}");
        assert_eq!(body.get("output_config"), None, "{id}");
        assert_eq!(body["temperature"], 0.4, "{id}");
        assert_eq!(body["tool_choice"], json!({"type": "any"}), "{id}");
        assert_eq!(response.unwrap().effort(), Some(Effort::Disabled), "{id}");
    }
}

#[test]
fn fable_rejects_explicit_off_before_transport() {
    let mut request = request();
    request.controls.generation.effort = Some(Effort::Disabled);
    assert!(matches!(resolve_controls(&request, &CLAUDE_FABLE_5),
        Err(ref error)
        if error.core().kind == ErrorKind::UnsupportedControl(ModelControl::Effort)
            && error.core().provider == "anthropic" && error.core().model_id == "claude-fable-5"
    ));
}

fn request_body(body: &JsonHttpBody) -> Value {
    serde_json::from_slice(body.as_json_bytes().expect("JSON request body"))
        .expect("valid JSON request body")
}
