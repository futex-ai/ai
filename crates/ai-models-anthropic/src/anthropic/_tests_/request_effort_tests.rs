//! Request effort overrides the default for one call on a shared model.

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest, ModelSpec};
use json_http::JsonHttpBody;
use serde_json::json;

use crate::{AnthropicModel, CLAUDE_FABLE_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_5, CLAUDE_SONNET_5};

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
    assert_eq!(
        requests[1].body.as_ref().unwrap()["thinking"],
        json!({"type": "disabled"})
    );
}

#[tokio::test]
async fn sonnet_5_low_matches_legacy_wire_request() {
    assert_legacy_wire_request(CLAUDE_SONNET_5, "claude-sonnet-5", Effort::Low, "low").await;
}

#[tokio::test]
async fn sonnet_5_medium_matches_legacy_wire_request() {
    assert_legacy_wire_request(CLAUDE_SONNET_5, "claude-sonnet-5", Effort::Medium, "medium").await;
}

#[tokio::test]
async fn opus_5_low_matches_legacy_wire_request() {
    assert_legacy_wire_request(CLAUDE_OPUS_5, "claude-opus-5", Effort::Low, "low").await;
}

#[tokio::test]
async fn opus_5_medium_matches_legacy_wire_request() {
    assert_legacy_wire_request(CLAUDE_OPUS_5, "claude-opus-5", Effort::Medium, "medium").await;
}

#[tokio::test]
async fn fable_5_low_matches_legacy_wire_request() {
    assert_legacy_wire_request(CLAUDE_FABLE_5, "claude-fable-5", Effort::Low, "low").await;
}

#[tokio::test]
async fn fable_5_medium_matches_legacy_wire_request() {
    assert_legacy_wire_request(CLAUDE_FABLE_5, "claude-fable-5", Effort::Medium, "medium").await;
}

async fn assert_legacy_wire_request(
    spec: ModelSpec,
    model_id: &str,
    effort: Effort,
    wire_effort: &str,
) {
    let body = json!({
        "content": [{"type": "text", "text": "Done"}],
        "stop_reason": "end_turn"
    });
    let (client, requests) = client_for_buffered_bodies(vec![body]);
    let model = AnthropicModel::new(client, spec, "key");
    let mut request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.effort = Some(effort);
    request.controls.generation.max_output_tokens = Some(256);
    request.controls.generation.temperature = Some(0.4);
    request.controls.generation.top_p = Some(0.8);

    let response = model.complete(&request).await;
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.body.clone())
            .collect::<Vec<_>>(),
        vec![Some(JsonHttpBody::Json(json!({
            "stream": true,
            "model": model_id,
            "max_tokens": 256,
            "system": "system",
            "messages": [{"role": "user", "content": [{"type": "text", "text": "hello"}]}],
            "thinking": {"type": "adaptive", "display": "omitted"},
            "output_config": {"effort": wire_effort}
        })))],
        "{model_id}/{wire_effort}: {response:?}"
    );
    assert_eq!(response.unwrap().effort(), Some(effort));
}
