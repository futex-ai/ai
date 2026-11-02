//! XAI portable generation-control tests.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelExecutionControls, ModelGenerationControls,
    ModelRequest, ModelToolChoice, ToolDefinition,
};
use ai_models_core::test_support::RecordedRequests;
use json_http::{JsonHttpClient, JsonHttpResponse};
use serde_json::json;

use super::{XaiModel, test_support::recording_http_client};

#[tokio::test]
async fn maps_generation_controls_timeout_and_blank_system_to_final_request() {
    let (http_client, requests) = recording_client();
    let model = XaiModel::new(http_client, "grok-4", "key");

    model
        .complete(&controlled_request())
        .await
        .expect("controlled response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request.body.as_ref().expect("request body");
    assert_eq!(request.timeout, Duration::from_secs(140));
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["top_p"], 0.85);
    assert_eq!(body["max_tokens"], 900);
    assert_eq!(body["stop"], json!(["done"]));
    assert_eq!(body["tool_choice"]["function"]["name"], "lookup");
    assert!(body.get("deferred").is_none());
}

#[tokio::test]
async fn required_or_auto_uses_forced_required_semantics() {
    let (http_client, requests) = recording_client();
    let model = XaiModel::new(http_client, "grok-4", "key");
    let mut request = controlled_request();
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
        let model = XaiModel::new(http_client, "grok-4", "key");
        let mut request = controlled_request();
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
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: vec![ToolDefinition {
            name: "lookup".to_owned(),
            description: "Look up a value".to_owned(),
            input_schema: json!({ "type": "object" }),
            activity_verb: None,
        }],
        response_schema: None,
        controls: ModelCallControls {
            generation: ModelGenerationControls {
                temperature: Some(0.2),
                top_p: Some(0.85),
                max_output_tokens: Some(900),
                stop_sequences: vec!["done".to_owned()],
                tool_choice: Some(ModelToolChoice::Function("lookup".to_owned())),
            },
            execution: ModelExecutionControls {
                total_timeout: Some(Duration::from_secs(140)),
                ..Default::default()
            },
        },
    }
}

fn recording_client() -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "Done", "tool_calls": [] }
            }]
        }),
    })
}
