//! Request effort and custom model limits at the shared engine boundary.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest, ModelSpec};
use ai_models_core::test_support::{fixtures_for_buffered_responses, recording_streaming_client};
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use crate::deepseek::{CUSTOM_DEFAULTS, DeepSeekModel, PROVIDER_ID};

#[tokio::test]
async fn request_effort_and_custom_output_ceiling_reach_transport() {
    let response = JsonHttpResponse {
        status: 200,
        body: json!({"choices": [{"finish_reason": "stop", "message": {"content": "Done"}}]}),
    };
    let (http, requests) = recording_streaming_client(fixtures_for_buffered_responses(vec![
        response.clone(),
        response,
    ]));
    let mut spec = ModelSpec::custom(PROVIDER_ID, "deepseek-custom", &CUSTOM_DEFAULTS);
    spec.max_output_tokens = 11;
    let model = DeepSeekModel::with_catalog_auth(http, spec, Arc::new(StaticHeaderAuth::default()));
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.max_output_tokens = Some(100);
    request.controls.generation.temperature = Some(0.4);
    for effort in [Effort::Disabled, Effort::Max] {
        request.controls.generation.effort = Some(effort);
        let response = model.complete(&request).await.expect("completion");
        assert_eq!(response.thinking_level.as_deref(), Some(effort.as_str()));
    }
    let requests = requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 2);
    let disabled = requests[0].body.as_ref().expect("disabled body");
    let enabled = requests[1].body.as_ref().expect("enabled body");
    assert_eq!(disabled["model"], "deepseek-custom");
    assert_eq!(disabled["thinking"], json!({"type": "disabled"}));
    assert_eq!(disabled["temperature"], 0.4);
    assert_eq!(disabled["max_tokens"], 11);
    assert_eq!(enabled["thinking"], json!({"type": "enabled"}));
    assert_eq!(enabled["reasoning_effort"], "max");
    assert!(enabled.get("temperature").is_none());
    assert_eq!(enabled["max_tokens"], 11);
}
