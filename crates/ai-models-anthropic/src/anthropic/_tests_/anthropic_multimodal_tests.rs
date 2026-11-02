//! Multimodal serialization tests for Anthropic.

use std::sync::Arc;

use ai_interface::{ConversationMessage, ConversationPart, Model, ModelRequest, ToolCall};
use json_http::{JsonHttpClient, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::Unimock;

use crate::CLAUDE_SONNET_4_6;
use crate::anthropic::stream_support::{RecordedRequests, client_for_buffered_bodies};

use super::AnthropicModel;

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client();
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "anthropic-key");

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
        .expect("Anthropic response should parse");

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
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "anthropic-key");

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

    let core = error.core();
    assert_eq!(core.kind, ai_interface::ErrorKind::UnsupportedInput);
    assert_eq!(core.provider, "anthropic");
    assert_eq!(core.model_id, "claude-sonnet-4-6");
    assert_eq!(
        core.message,
        "Anthropic accepts text and image content parts only"
    );
}

fn recording_http_client() -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    client_for_buffered_bodies(vec![json!({
        "stop_reason": "end_turn",
        "content": [{ "type": "text", "text": "Done" }],
        "usage": { "input_tokens": 1, "output_tokens": 1 }
    })])
}
