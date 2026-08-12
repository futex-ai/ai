//! MiniMax portable call-control tests.

use std::sync::Arc;
use std::time::Duration;

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice,
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
    tool_request.controls.generation.tool_choice = Some(ModelToolChoice::Required);
    let tool_error = model
        .complete(&tool_request)
        .await
        .expect_err("required tool choice should fail");
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
