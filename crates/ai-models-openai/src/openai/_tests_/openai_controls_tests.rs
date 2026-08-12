//! OpenAI portable call-control tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::OpenAiModel;

#[tokio::test]
async fn maps_non_reasoning_controls_and_timeout_to_the_final_request() {
    let (http_client, requests) = recording_client();
    let model = OpenAiModel::new(http_client, "gpt-5.5-mini", "key");

    model
        .complete(&controlled_request())
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.timeout, Duration::from_secs(90));
    assert!(body.get("instructions").is_none());
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["top_p"], 0.8);
    assert_eq!(body["max_output_tokens"], 700);
    assert_eq!(body["tool_choice"]["type"], "function");
    assert_eq!(body["tool_choice"]["name"], "lookup");
}

#[tokio::test]
async fn reasoning_models_keep_sampling_provider_fixed() {
    let (http_client, requests) = recording_client();
    let model = OpenAiModel::with_catalog_auth(
        http_client,
        "gpt-5.6-sol",
        "gpt-5.6",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::bearer_token("key")),
    );

    model
        .complete(&controlled_request())
        .await
        .expect("reasoning response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let body = requests[0].body.as_ref().expect("request body");
    assert!(body.get("temperature").is_none());
    assert!(body.get("top_p").is_none());
    assert_eq!(body["max_output_tokens"], 700);
}

#[tokio::test]
async fn required_or_auto_uses_forced_required_semantics() {
    let (http_client, requests) = recording_client();
    let model = OpenAiModel::new(http_client, "gpt-5.5-mini", "key");
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
        let (http_client, requests) = recording_client();
        let model = OpenAiModel::new(http_client, "gpt-5.5-mini", "key");
        let mut request = base_request();
        request.system_prompt = system_prompt.to_owned();

        model
            .complete(&request)
            .await
            .expect("response should parse");

        let requests = requests.lock().expect("request lock should be available");
        let body = requests[0].body.as_ref().expect("request body");
        match expected {
            Some(expected) => assert_eq!(body["instructions"], expected),
            None => assert!(body.get("instructions").is_none()),
        }
    }
}

#[tokio::test]
async fn rejects_stops_and_required_deferred_mode_before_transport() {
    let model = OpenAiModel::new(no_call_client(), "gpt-5.5-mini", "key");
    let mut stop_request = base_request();
    stop_request.controls.generation.stop_sequences = vec!["stop".to_owned()];
    let stop_error = model
        .complete(&stop_request)
        .await
        .expect_err("Responses stops should be rejected");
    assert!(matches!(
        stop_error,
        ModelError::UnsupportedControl {
            control: ModelControl::StopSequences,
            ..
        }
    ));

    let mut deferred_request = base_request();
    deferred_request.controls.execution.completion_mode = ModelCompletionMode::RequireDeferred;
    let deferred_error = model
        .complete(&deferred_request)
        .await
        .expect_err("required deferred mode should be rejected");
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
    request.tools = vec![ToolDefinition {
        name: "lookup".to_owned(),
        description: "Look up a value".to_owned(),
        input_schema: json!({ "type": "object" }),
        activity_verb: None,
    }];
    request.controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.2),
            top_p: Some(0.8),
            max_output_tokens: Some(700),
            stop_sequences: Vec::new(),
            tool_choice: Some(ModelToolChoice::Function("lookup".to_owned())),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(90)),
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
            .next_call(matching!(_))
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
                            "status": "completed",
                            "output": [{
                                "type": "message",
                                "role": "assistant",
                                "content": [{ "type": "output_text", "text": "Done" }]
                            }]
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
