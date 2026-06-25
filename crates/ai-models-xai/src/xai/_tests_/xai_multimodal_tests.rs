//! Multimodal serialization tests for xAI.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{ConversationContentPart, ConversationMessage, Model, ModelRequest, ToolCall};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::XaiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client();
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::assistant(
                    "looking",
                    vec![ToolCall {
                        id: "call-view".to_owned(),
                        name: "attachment_view".to_owned(),
                        input: json!({"attachment_id": "attachment://image-id"}),
                        operation_id: None,
                    }],
                ),
                ConversationMessage::tool(
                    "{\"context_accepted\":true}",
                    "attachment_view",
                    "call-view",
                ),
                ConversationMessage::user_with_parts(
                    "Visual context is available.",
                    vec![
                        ConversationContentPart::Text {
                            text: "Visual context is available.".to_owned(),
                        },
                        ConversationContentPart::Image {
                            mime_type: "image/png".to_owned(),
                            data_base64: "abc123".to_owned(),
                        },
                    ],
                ),
            ],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("xAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let messages = &requests[0].body.as_ref().expect("body present")["messages"];
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[3]["content"][0]["type"], "text");
    assert_eq!(messages[3]["content"][1]["type"], "image_url");
    assert_eq!(
        messages[3]["content"][1]["image_url"]["url"],
        "data:image/png;base64,abc123"
    );
}

fn recording_http_client() -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "Done", "tool_calls": [] }
            }]
        }),
    }])));
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
