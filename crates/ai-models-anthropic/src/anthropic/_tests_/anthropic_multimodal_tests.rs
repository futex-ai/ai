//! Multimodal serialization tests for Anthropic.

use std::sync::Arc;

use ai_interface::{
    ConversationContentPart, ConversationMessage, Model, ModelError, ModelRequest, ToolCall,
};
use json_http::{JsonHttpClient, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::Unimock;

use super::AnthropicModel;
use crate::anthropic::stream_support::{RecordedRequests, client_for_buffered_bodies};

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
            controls: Default::default(),
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

#[tokio::test]
async fn rejects_video_content_parts_before_transport() {
    let http_client: Arc<dyn JsonHttpClient> = Arc::new(TransportBackedJsonHttpClient::new(
        Arc::new(Unimock::new(())),
    ));
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "anthropic-key");

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
            assert_eq!(provider, "anthropic");
            assert_eq!(model_id, "claude-sonnet-4-6");
            assert_eq!(
                message,
                "Anthropic accepts text and image content parts only"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn recording_http_client() -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    client_for_buffered_bodies(vec![json!({
        "stop_reason": "end_turn",
        "content": [{ "type": "text", "text": "Done" }],
        "usage": { "input_tokens": 1, "output_tokens": 1 }
    })])
}
