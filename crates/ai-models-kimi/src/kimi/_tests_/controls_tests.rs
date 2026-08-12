//! Kimi portable call-control tests.

use std::time::Duration;

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice,
};
use serde_json::json;

use super::{
    KimiModel,
    test_support::{recording_http_client, successful_response, unused_http_client},
};

#[tokio::test]
async fn maps_supported_controls_and_keeps_k3_sampling_fixed() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = KimiModel::new(http_client, "key");

    model
        .complete(&controlled_request())
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.timeout, Duration::from_secs(180));
    assert_eq!(body["messages"][0]["role"], "user");
    assert!(body.get("temperature").is_none());
    assert!(body.get("top_p").is_none());
    assert_eq!(body["max_completion_tokens"], 1000);
    assert_eq!(body["stop"], json!(["first", "second"]));
    assert_eq!(body["tool_choice"]["type"], "function");
    assert_eq!(body["tool_choice"]["function"]["name"], "lookup");
}

#[tokio::test]
async fn rejects_required_deferred_mode_before_transport() {
    let model = KimiModel::new(unused_http_client(), "key");
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
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = KimiModel::new(http_client, "key");
    let mut request = base_request();
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);

    model
        .complete(&request)
        .await
        .expect("required fallback should use provider enforcement");

    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(
        requests[0].body.as_ref().expect("request body")["tool_choice"],
        "required"
    );
}

#[tokio::test]
async fn system_prompt_omits_empty_and_spaces_but_preserves_nonblank() {
    for (system_prompt, expected) in [("", None), ("   ", None), ("normal", Some("normal"))] {
        let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
        let model = KimiModel::new(http_client, "key");
        let mut request = base_request();
        request.system_prompt = system_prompt.to_owned();

        model
            .complete(&request)
            .await
            .expect("response should parse");

        let requests = requests.lock().expect("request lock should be available");
        let messages = &requests[0].body.as_ref().expect("request body")["messages"];
        match expected {
            Some(expected) => assert_eq!(messages[0]["content"], expected),
            None => assert_eq!(messages[0]["role"], "user"),
        }
    }
}

fn controlled_request() -> ModelRequest {
    let mut request = base_request();
    request.controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.2),
            top_p: Some(0.8),
            max_output_tokens: Some(1000),
            stop_sequences: vec!["first".to_owned(), "second".to_owned()],
            tool_choice: Some(ModelToolChoice::Function("lookup".to_owned())),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(180)),
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
