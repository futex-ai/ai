//! Multimodal serialization tests for xAI.

use ai_interface::{
    ConversationContentPart, ConversationMessage, Model, ModelError, ModelRequest, ToolCall,
};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    XaiModel,
    test_support::{recording_http_client, unused_http_client},
};

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client(successful_response());
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
            controls: Default::default(),
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

#[tokio::test]
async fn rejects_video_content_parts_before_transport() {
    let model = XaiModel::new(unused_http_client(), "grok-4", "xai-key");

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
            assert_eq!(provider, "xai");
            assert_eq!(model_id, "grok-4");
            assert_eq!(message, "xAI accepts text and image content parts only");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn successful_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "Done", "tool_calls": [] }
            }]
        }),
    }
}
