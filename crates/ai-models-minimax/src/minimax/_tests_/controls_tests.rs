//! MiniMax portable call-control tests.

use std::sync::Arc;
use std::time::Duration;

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice, ToolDefinition,
};
use json_http::{JsonHttpClient, JsonHttpResponse, TransportBackedJsonHttpClient};
use serde_json::{Value, json};
use unimock::Unimock;

use super::{MiniMaxModel, support::recording_http_client};

#[tokio::test]
async fn maps_supported_controls_and_blank_system_to_the_final_request() {
    let (http_client, requests) = recording_http_client([successful_response()]);
    let model = MiniMaxModel::new(http_client, "MiniMax-M3", "key");

    model
        .complete(&controlled_request())
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.timeout, Duration::from_secs(110));
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["temperature"], 0.3);
    assert_eq!(body["top_p"], 0.7);
    assert_eq!(body["max_completion_tokens"], 1000);
    assert_eq!(body["tool_choice"], "none");
}

#[tokio::test]
async fn rejects_unsupported_controls_before_transport() {
    let model = MiniMaxModel::new(unused_http_client(), "MiniMax-M3", "key");
    let mut stop_request = base_request();
    stop_request.controls.generation.stop_sequences = vec!["stop".to_owned()];
    let stop_error = model
        .complete(&stop_request)
        .await
        .expect_err("stops should fail");
    assert!(matches!(
        stop_error,
        ModelError::UnsupportedControl {
            control: ModelControl::StopSequences,
            ..
        }
    ));

    let mut tool_request = base_request();
    tool_request.controls.generation.tool_choice =
        Some(ModelToolChoice::Function("lookup".to_owned()));
    let tool_error = model
        .complete(&tool_request)
        .await
        .expect_err("named tool choice should fail");
    assert!(matches!(
        tool_error,
        ModelError::UnsupportedControl {
            control: ModelControl::ToolChoice,
            ..
        }
    ));

    let mut deferred_request = base_request();
    deferred_request.controls.execution.completion_mode = ModelCompletionMode::RequireDeferred;
    let deferred_error = model
        .complete(&deferred_request)
        .await
        .expect_err("required deferred mode should fail");
    assert!(matches!(
        deferred_error,
        ModelError::UnsupportedControl {
            control: ModelControl::CompletionMode,
            ..
        }
    ));
}

#[tokio::test]
async fn minimax_m3_serializes_strict_and_fallback_required_as_required() {
    let (http_client, requests) =
        recording_http_client([successful_response(), successful_response()]);
    let model = MiniMaxModel::new(http_client, "MiniMax-M3", "key");

    for choice in [ModelToolChoice::Required, ModelToolChoice::RequiredOrAuto] {
        let mut request = base_request();
        request.tools = vec![lookup_tool()];
        request.controls.generation.tool_choice = Some(choice);
        model
            .complete(&request)
            .await
            .expect("MiniMax-M3 required tool choice should be supported");
    }

    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        let body = request.body.as_ref().expect("request body");
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["tools"][0]["function"]["name"], "lookup");
    }
}

#[tokio::test]
async fn fallback_uses_auto_for_minimax_models_without_verified_required_support() {
    let (http_client, requests) = recording_http_client([successful_response()]);
    let model = MiniMaxModel::new(http_client, "MiniMax-M2.7", "key");
    let mut request = base_request();
    request.tools = vec![lookup_tool()];
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);

    model
        .complete(&request)
        .await
        .expect("fallback should use documented automatic semantics");

    let requests = requests.lock().expect("request lock should be available");
    let body = requests[0].body.as_ref().expect("request body");
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["tools"][0]["function"]["name"], "lookup");
}

#[tokio::test]
async fn strict_required_remains_unsupported_for_other_minimax_models() {
    let model = MiniMaxModel::new(unused_http_client(), "MiniMax-M2.7", "key");
    let mut request = base_request();
    request.controls.generation.tool_choice = Some(ModelToolChoice::Required);

    let error = model
        .complete(&request)
        .await
        .expect_err("unverified strict required support should fail");

    assert!(matches!(
        error,
        ModelError::UnsupportedControl {
            control: ModelControl::ToolChoice,
            ..
        }
    ));
}

#[tokio::test]
async fn system_prompt_omits_empty_and_spaces_but_preserves_nonblank() {
    for (system_prompt, expected) in [("", None), ("   ", None), ("normal", Some("normal"))] {
        let (http_client, requests) = recording_http_client([successful_response()]);
        let model = MiniMaxModel::new(http_client, "MiniMax-M3", "key");
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
            temperature: Some(0.3),
            top_p: Some(0.7),
            max_output_tokens: Some(1000),
            stop_sequences: Vec::new(),
            tool_choice: Some(ModelToolChoice::None),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(110)),
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

fn unused_http_client() -> Arc<dyn JsonHttpClient> {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}

fn successful_response() -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "Done" }
            }]
        }),
    }
}
