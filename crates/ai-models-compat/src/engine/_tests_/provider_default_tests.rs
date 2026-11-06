//! Provider defaults and explicit off controls at the Chat Completions boundary.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, Effort, ErrorKind, Model, ModelControl, ModelRequest, ModelSpec,
};
use ai_models_core::test_support::{fixtures_for_buffered_responses, recording_streaming_client};
use json_http::{JsonHttpBody, JsonHttpResponse, StaticHeaderAuth, TransportBackedJsonHttpClient};
use serde_json::{Value, json};
use unimock::Unimock;

use crate::kimi::{KIMI_K3, KimiModel};
use crate::xai::{CUSTOM_DEFAULTS, GROK_4_20, PROVIDER_ID, XaiModel};

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({"choices": [{"finish_reason": "stop", "message": {"content": "Done"}}]}),
    }
}

#[tokio::test]
async fn provider_default_omits_reasoning_and_reports_no_effort() {
    for spec in [
        GROK_4_20,
        ModelSpec::custom(PROVIDER_ID, "grok-custom", &CUSTOM_DEFAULTS),
    ] {
        let (http, requests) =
            recording_streaming_client(fixtures_for_buffered_responses(vec![response()]));
        let model = XaiModel::new(http, spec, "key");
        let response = model.complete(&request()).await.unwrap();
        assert_eq!(response.effort(), None);
        let requests = requests.lock().unwrap();
        assert_eq!(
            request_body(requests[0].body.as_ref().unwrap()).get("reasoning_effort"),
            None
        );
        assert_eq!(
            request_body(requests[0].body.as_ref().unwrap()).get("thinking"),
            None
        );
    }
}

#[tokio::test]
async fn explicit_off_sends_none_for_custom_grok() {
    for model_id in ["grok-4.3", "grok-4.20-custom"] {
        let (http, requests) =
            recording_streaming_client(fixtures_for_buffered_responses(vec![response()]));
        let spec = ModelSpec::custom(PROVIDER_ID, model_id, &CUSTOM_DEFAULTS);
        let model = XaiModel::new(http, spec, "key");
        let mut request = request();
        request.controls.generation.effort = Some(Effort::Disabled);
        let response = model.complete(&request).await.unwrap();
        assert_eq!(
            request_body(requests.lock().unwrap()[0].body.as_ref().unwrap())["reasoning_effort"],
            "none"
        );
        assert_eq!(response.effort(), Some(Effort::Disabled));
    }
}

#[tokio::test]
async fn kimi_rejects_off_even_when_a_custom_spec_advertises_it() {
    let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))));
    let mut spec = KIMI_K3;
    spec.efforts = spec.efforts.with(Effort::Disabled);
    let model = KimiModel::with_catalog_auth(http, spec, Arc::new(StaticHeaderAuth::default()));
    let mut request = request();
    request.controls.generation.effort = Some(Effort::Disabled);
    assert!(matches!(model.complete(&request).await,
        Err(ref error)
        if error.core().kind == ErrorKind::UnsupportedControl(ModelControl::Effort)
            && error.core().provider == "kimi" && error.core().model_id == "kimi-k3"
    ));
}

fn request_body(body: &JsonHttpBody) -> Value {
    serde_json::from_slice(body.as_json_bytes().expect("JSON request body"))
        .expect("valid JSON request body")
}
