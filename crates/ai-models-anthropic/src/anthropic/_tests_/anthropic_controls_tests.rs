//! Anthropic portable call-control tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice,
};
use ai_models_core::ThinkingLevel;
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::AnthropicModel;

#[tokio::test]
async fn maps_controls_bounds_output_and_keeps_thinking_sampling_fixed() {
    let (http_client, requests) = recording_client();
    let disabled = AnthropicModel::new(http_client.clone(), "claude-sonnet-4-6", "key");
    let thinking = AnthropicModel::with_catalog_auth(
        http_client,
        "claude-sonnet-5-thinking-high",
        "claude-sonnet-5",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::new(Default::default())),
    );

    disabled
        .complete(&controlled_request())
        .await
        .expect("non-thinking response should parse");
    thinking
        .complete(&controlled_request())
        .await
        .expect("thinking response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let plain = requests[0].body.as_ref().expect("plain body");
    assert_eq!(requests[0].timeout, Duration::from_secs(75));
    assert!(plain.get("system").is_none());
    assert_eq!(plain["temperature"], 0.3);
    assert_eq!(plain["top_p"], 0.7);
    assert_eq!(plain["max_tokens"], 4096);
    assert_eq!(plain["stop_sequences"], json!(["first", "second"]));
    assert_eq!(plain["tool_choice"]["type"], "tool");
    assert_eq!(plain["tool_choice"]["name"], "lookup");
    let thinking = requests[1].body.as_ref().expect("thinking body");
    assert!(thinking.get("temperature").is_none());
    assert!(thinking.get("top_p").is_none());
    assert_eq!(thinking["max_tokens"], 4096);
}

#[tokio::test]
async fn rejects_required_deferred_mode_before_transport() {
    let model = AnthropicModel::new(no_call_client(), "claude-sonnet-4-6", "key");
    let mut request = base_request();
    request.controls.execution.completion_mode = ModelCompletionMode::RequireDeferred;

    let error = model
        .complete(&request)
        .await
        .expect_err("required deferred mode should fail");

    assert!(matches!(
        error,
        ModelError::UnsupportedControl {
            control: ModelControl::CompletionMode,
            ..
        }
    ));
}

#[tokio::test]
async fn required_or_auto_uses_forced_required_semantics() {
    let (http_client, requests) = recording_client();
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");
    let mut request = base_request();
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);

    model
        .complete(&request)
        .await
        .expect("required fallback should use provider enforcement");

    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(
        requests[0].body.as_ref().expect("request body")["tool_choice"]["type"],
        "any"
    );
}

#[tokio::test]
async fn system_prompt_omits_empty_and_spaces_but_preserves_nonblank() {
    for (system_prompt, expected) in [("", None), ("   ", None), ("normal", Some("normal"))] {
        let (http_client, requests) = recording_client();
        let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");
        let mut request = base_request();
        request.system_prompt = system_prompt.to_owned();

        model
            .complete(&request)
            .await
            .expect("response should parse");

        let requests = requests.lock().expect("request lock should be available");
        let body = requests[0].body.as_ref().expect("request body");
        match expected {
            Some(expected) => assert_eq!(body["system"], expected),
            None => assert!(body.get("system").is_none()),
        }
    }
}

fn controlled_request() -> ModelRequest {
    let mut request = base_request();
    request.controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.3),
            top_p: Some(0.7),
            max_output_tokens: Some(9000),
            stop_sequences: vec!["first".to_owned(), "second".to_owned()],
            tool_choice: Some(ModelToolChoice::Function("lookup".to_owned())),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(75)),
            completion_mode: ModelCompletionMode::PreferDeferred,
        },
    };
    request
}

fn base_request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn recording_client() -> (Arc<dyn JsonHttpClient>, Arc<Mutex<Vec<JsonHttpRequest>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("request lock should be available")
                        .push(request.clone());
                    Ok(JsonHttpResponse {
                        status: 200,
                        body: json!({
                            "content": [{ "type": "text", "text": "Done" }],
                            "stop_reason": "end_turn"
                        }),
                    })
                })
            }),
    ));
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn no_call_client() -> Arc<dyn JsonHttpClient> {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}
