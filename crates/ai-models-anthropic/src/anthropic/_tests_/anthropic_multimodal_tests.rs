//! Multimodal serialization tests for Anthropic.

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

use super::AnthropicModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client();
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "anthropic-key");

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
        .expect("Anthropic response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let messages = &requests[0].body.as_ref().expect("body present")["messages"];
    assert_eq!(messages[1]["content"][0]["type"], "tool_result");
    assert_eq!(messages[1]["content"][0]["tool_use_id"], "call-view");
    assert_eq!(messages[1]["content"][1]["type"], "text");
    assert_eq!(messages[1]["content"][2]["type"], "image");
    assert_eq!(messages[1]["content"][2]["source"]["type"], "base64");
    assert_eq!(
        messages[1]["content"][2]["source"]["media_type"],
        "image/png"
    );
    assert_eq!(messages[1]["content"][2]["source"]["data"], "abc123");
}

fn recording_http_client() -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([JsonHttpResponse {
        status: 200,
        body: json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Done" }],
            "usage": { "input_tokens": 1, "output_tokens": 1 }
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
