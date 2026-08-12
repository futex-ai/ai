//! Qwen portable call-control tests.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice,
};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::QWEN_3_7_PLUS;

use super::{
    QwenModel,
    test_support::{recording_http_client, successful_response, unused_http_client},
};

#[tokio::test]
async fn maps_non_thinking_controls_to_the_final_request() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = QwenModel::with_catalog_auth(
        http_client,
        "qwen3.7-plus-thinking-disabled",
        QWEN_3_7_PLUS,
        ThinkingLevel::Disabled,
        Arc::new(StaticHeaderAuth::bearer_token("key")),
    )
    .expect("configuration should be supported");

    model
        .complete(&controlled_request(ModelToolChoice::Function(
            "lookup".to_owned(),
        )))
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.timeout, Duration::from_secs(100));
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["temperature"], 0.25);
    assert_eq!(body["top_p"], 0.75);
    assert_eq!(body["max_completion_tokens"], 800);
    assert_eq!(body["stop"], json!(["done"]));
    assert_eq!(body["tool_choice"]["function"]["name"], "lookup");
}

#[tokio::test]
async fn thinking_keeps_sampling_fixed_and_preserves_auto_tool_choice() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = QwenModel::new(http_client, "key");

    model
        .complete(&controlled_request(ModelToolChoice::Auto))
        .await
        .expect("thinking response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let body = requests[0].body.as_ref().expect("request body");
    assert!(body.get("temperature").is_none());
    assert!(body.get("top_p").is_none());
    assert_eq!(body["max_completion_tokens"], 800);
    assert_eq!(body["stop"], json!(["done"]));
    assert_eq!(body["tool_choice"], "auto");
}

#[tokio::test]
async fn rejects_unsupported_tool_and_completion_choices_before_transport() {
    let model = QwenModel::new(unused_http_client(), "key");
    for choice in [
        ModelToolChoice::Required,
        ModelToolChoice::Function("lookup".to_owned()),
    ] {
        let error = model
            .complete(&controlled_request(choice))
            .await
            .expect_err("forced thinking choice should fail");
        assert!(matches!(
            error,
            ModelError::UnsupportedControl {
                control: ModelControl::ToolChoice,
                ..
            }
        ));
    }

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

fn controlled_request(tool_choice: ModelToolChoice) -> ModelRequest {
    let mut request = base_request();
    request.controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.25),
            top_p: Some(0.75),
            max_output_tokens: Some(800),
            stop_sequences: vec!["done".to_owned()],
            tool_choice: Some(tool_choice),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(100)),
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
