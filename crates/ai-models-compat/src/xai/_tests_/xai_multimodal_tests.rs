//! Multimodal serialization tests for xAI.

use ai_interface::ErrorKind;

use ai_interface::{ConversationMessage, ConversationPart, Model, ModelRequest, ToolCall};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::test_support::custom_spec;
use super::{
    XaiModel,
    test_support::{recording_http_client, unused_http_client},
};

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client(successful_response());
    let model = XaiModel::new(http_client, custom_spec("grok-4"), "xai-key");

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
                ConversationMessage::user_with_parts(vec![
                    ConversationPart::Text {
                        text: "Visual context is available.".to_owned(),
                        provider_state: None,
                    },
                    ConversationPart::Image {
                        mime_type: "image/png".to_owned(),
                        data_base64: "abc123".to_owned(),
                        provider_state: None,
                    },
                ]),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("xAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let messages = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["messages"];
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[3]["content"][0]["type"], "text");
    assert_eq!(messages[3]["content"][1]["type"], "image_url");
    assert_eq!(
        messages[3]["content"][1]["image_url"]["url"],
        "data:image/png;base64,abc123"
    );
}

#[tokio::test]
async fn rejects_video_content_parts_before_transport() {
    let model = XaiModel::new(unused_http_client(), custom_spec("grok-4"), "xai-key");

    let error = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user_with_parts(vec![
                ConversationPart::Video {
                    mime_type: "video/mp4".to_owned(),
                    data_base64: "dmlkZW8=".to_owned(),
                    provider_state: None,
                },
            ])],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect_err("video parts must be rejected");

    assert_eq!(error.core().kind, ErrorKind::UnsupportedInput);

    assert_eq!(error.core().provider, "xai");
    assert_eq!(error.core().model_id, "grok-4");
    assert_eq!(
        error.core().message,
        "xAI accepts text and image content parts only"
    );
}

fn successful_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "Done", "tool_calls": [] }
            }]
        }),
    }
}
