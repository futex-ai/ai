//! DeepSeek portable call-control tests.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::DEEPSEEK_V4_PRO;

use super::{
    DeepSeekModel,
    test_support::{recording_http_client, successful_response, unused_http_client},
};

#[tokio::test]
async fn maps_disabled_thinking_controls_to_the_final_request() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = DeepSeekModel::with_catalog_auth(
        http_client,
        "deepseek-v4-pro-thinking-disabled",
        DEEPSEEK_V4_PRO,
        ThinkingLevel::Disabled,
        Arc::new(StaticHeaderAuth::bearer_token("key")),
    )
    .expect("configuration should be supported");

    model
        .complete(&controlled_request(ModelToolChoice::Required))
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.url, "https://api.deepseek.com/chat/completions");
    assert_eq!(request.timeout, Duration::from_secs(600));
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["top_p"], 0.8);
    assert_eq!(body["max_tokens"], 1200);
    assert_eq!(body["stop"], json!(["done"]));
    assert_eq!(body["tool_choice"], "required");
}

#[tokio::test]
async fn thinking_keeps_sampling_fixed_and_uses_native_auto_tool_semantics() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = DeepSeekModel::new(http_client, "key");

    model
        .complete(&controlled_request(ModelToolChoice::Auto))
        .await
        .expect("thinking response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let body = requests[0].body.as_ref().expect("request body");
    assert!(body.get("temperature").is_none());
    assert!(body.get("top_p").is_none());
    assert_eq!(body["max_tokens"], 1200);
    assert_eq!(body["stop"], json!(["done"]));
    assert!(body.get("tool_choice").is_none());
}

#[tokio::test]
async fn thinking_required_or_auto_preserves_tools_and_omits_tool_choice() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = DeepSeekModel::new(http_client, "key");
    let mut request = controlled_request(ModelToolChoice::RequiredOrAuto);
    request.tools = vec![lookup_tool()];

    model
        .complete(&request)
        .await
        .expect("fallback tool choice should be supported");

    let requests = requests.lock().expect("request lock should be available");
    let body = requests[0].body.as_ref().expect("request body");
    assert!(body.get("tool_choice").is_none());
    assert_eq!(body["tools"][0]["function"]["name"], "lookup");
}

#[tokio::test]
async fn disabled_thinking_required_or_auto_still_forces_tool_use() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = DeepSeekModel::with_catalog_auth(
        http_client,
        "deepseek-v4-pro-thinking-disabled",
        DEEPSEEK_V4_PRO,
        ThinkingLevel::Disabled,
        Arc::new(StaticHeaderAuth::bearer_token("key")),
    )
    .expect("configuration should be supported");

    model
        .complete(&controlled_request(ModelToolChoice::RequiredOrAuto))
        .await
        .expect("non-thinking fallback should retain required semantics");

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
        let model = DeepSeekModel::new(http_client, "key");
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

#[tokio::test]
async fn rejects_forced_thinking_tools_and_required_deferred_before_transport() {
    let model = DeepSeekModel::new(unused_http_client(), "key");
    for choice in [
        ModelToolChoice::Required,
        ModelToolChoice::Function("lookup".to_owned()),
    ] {
        let error = model
            .complete(&controlled_request(choice))
            .await
            .expect_err("forced thinking tool choice should fail");
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
    request.tools = vec![lookup_tool()];
    request.controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.2),
            top_p: Some(0.8),
            max_output_tokens: Some(1200),
            stop_sequences: vec!["done".to_owned()],
            tool_choice: Some(tool_choice),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(600)),
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

fn lookup_tool() -> ToolDefinition {
    ToolDefinition {
        name: "lookup".to_owned(),
        description: "Look up a value".to_owned(),
        input_schema: json!({"type": "object"}),
        activity_verb: None,
    }
}
