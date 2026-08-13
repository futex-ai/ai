//! Multimodal serialization tests for OpenAI.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{
    ConversationContentPart, ConversationMessage, Model, ModelError, ModelRequest, ToolCall,
};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::OpenAiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn serializes_multimodal_messages_and_tool_history() {
    let (http_client, requests) = recording_http_client(openai_text_response("Done"));
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user_with_parts(
                    "see image",
                    vec![
                        ConversationContentPart::Text {
                            text: "look".to_owned(),
                        },
                        ConversationContentPart::Image {
                            mime_type: "image/png".to_owned(),
                            data_base64: "abc123".to_owned(),
                        },
                    ],
                ),
                ConversationMessage::assistant(
                    "checking",
                    vec![ToolCall {
                        id: "call_1".to_owned(),
                        name: "memory_read".to_owned(),
                        input: json!({"path": "root"}),
                        operation_id: None,
                    }],
                ),
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &requests[0].body.as_ref().expect("body present")["input"];
    assert_eq!(input[0]["content"][0]["type"], "input_text");
    assert_eq!(input[0]["content"][0]["text"], "look");
    assert_eq!(input[0]["content"][1]["type"], "input_image");
    assert_eq!(
        input[0]["content"][1]["image_url"],
        "data:image/png;base64,abc123"
    );
    assert_eq!(input[1]["role"], "assistant");
    assert_eq!(input[1]["content"], "checking");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["call_id"], "call_1");
    assert_eq!(input[2]["name"], "memory_read");
    assert_eq!(input[2]["arguments"], "{\"path\":\"root\"}");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(input[3]["call_id"], "call_1");
    assert_eq!(input[3]["output"], "{\"ok\":true}");
}

#[tokio::test]
async fn rejects_video_content_parts_before_transport() {
    let http_client: Arc<dyn JsonHttpClient> = Arc::new(TransportBackedJsonHttpClient::new(
        Arc::new(Unimock::new(())),
    ));
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");

    let error = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user_with_parts(
                "fallback",
                vec![ConversationContentPart::Video {
                    mime_type: "video/mp4".to_owned(),
                    data_base64: "dmlkZW8=".to_owned(),
                }],
            )],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect_err("video parts must be rejected");

    match error {
        ModelError::Provider {
            provider,
            model_id,
            message,
        } => {
            assert_eq!(provider, "openai");
            assert_eq!(model_id, "gpt-5.5");
            assert_eq!(message, "OpenAI accepts text and image content parts only");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([response])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let responses = responses.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(responses
                        .lock()
                        .expect("responses lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected transport call"))
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn openai_text_response(text: &str) -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": text }]
            }],
            "usage": {
                "input_tokens": 120,
                "output_tokens": 32,
                "total_tokens": 152
            }
        }),
    }
}
