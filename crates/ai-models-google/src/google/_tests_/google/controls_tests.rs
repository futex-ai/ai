//! Google portable call-control tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelControl, ModelError,
    ModelExecutionControls, ModelGenerationControls, ModelRequest, ModelToolChoice, ToolDefinition,
};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::super::GoogleModel;

#[tokio::test]
async fn maps_controls_and_preserves_full_function_json_schema() {
    let (http_client, requests) = recording_client();
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    model
        .complete(&controlled_request())
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.timeout, Duration::from_secs(120));
    assert!(body.get("systemInstruction").is_none());
    assert_eq!(body["generationConfig"]["temperature"], 0.4);
    assert_eq!(body["generationConfig"]["topP"], 0.9);
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 600);
    assert_eq!(
        body["generationConfig"]["stopSequences"],
        json!(["first", "second"])
    );
    assert_eq!(
        body["tools"][0]["functionDeclarations"][0]["parametersJsonSchema"]["uniqueItems"],
        true
    );
    assert!(
        body["tools"][0]["functionDeclarations"][0]
            .get("parameters")
            .is_none()
    );
    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
    assert_eq!(
        body["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"],
        json!(["lookup"])
    );
}

#[tokio::test]
async fn rejects_required_deferred_mode_before_transport() {
    let model = GoogleModel::new(no_call_client(), "gemini-3.6-flash", "key");
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
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
    let mut request = base_request();
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);

    model
        .complete(&request)
        .await
        .expect("required fallback should use provider enforcement");

    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(
        requests[0].body.as_ref().expect("request body")["toolConfig"]["functionCallingConfig"]["mode"],
        "ANY"
    );
}

#[tokio::test]
async fn system_prompt_omits_empty_and_spaces_but_preserves_nonblank() {
    for (system_prompt, expected) in [("", None), ("   ", None), ("normal", Some("normal"))] {
        let (http_client, requests) = recording_client();
        let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
        let mut request = base_request();
        request.system_prompt = system_prompt.to_owned();

        model
            .complete(&request)
            .await
            .expect("response should parse");

        let requests = requests.lock().expect("request lock should be available");
        let body = requests[0].body.as_ref().expect("request body");
        match expected {
            Some(expected) => {
                assert_eq!(body["systemInstruction"]["parts"][0]["text"], expected)
            }
            None => assert!(body.get("systemInstruction").is_none()),
        }
    }
}

fn controlled_request() -> ModelRequest {
    let mut request = base_request();
    request.tools = vec![ToolDefinition {
        name: "lookup".to_owned(),
        description: "Look up values".to_owned(),
        input_schema: json!({
            "type": "array",
            "items": { "type": "string" },
            "uniqueItems": true
        }),
        activity_verb: None,
    }];
    request.controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.4),
            top_p: Some(0.9),
            max_output_tokens: Some(600),
            stop_sequences: vec!["first".to_owned(), "second".to_owned()],
            tool_choice: Some(ModelToolChoice::Function("lookup".to_owned())),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(120)),
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
                            "candidates": [{
                                "finishReason": "STOP",
                                "content": { "parts": [{ "text": "Done" }] }
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
