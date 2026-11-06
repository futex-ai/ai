//! Multimodal serialization tests for OpenAI.

use std::sync::Arc;

use ai_interface::{ConversationMessage, ConversationPart, Model, ModelRequest, ToolCall};
use json_http::{JsonHttpClient, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::Unimock;

use crate::openai::stream_support::client_for_buffered_bodies;
use crate::openai::stream_support::custom_spec;

use super::OpenAiModel;

#[tokio::test]
async fn serializes_multimodal_messages_and_tool_history() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "sk-openai");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user_with_parts(vec![
                    ConversationPart::Text {
                        text: "look".to_owned(),
                        provider_state: None,
                    },
                    ConversationPart::Image {
                        mime_type: "image/png".to_owned(),
                        data_base64: "abc123".to_owned(),
                        provider_state: None,
                    },
                ]),
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
    let input = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["input"];
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
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "sk-openai");

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
    assert_eq!(core.provider, "openai");
    assert_eq!(core.model_id, "gpt-5.5");
    assert_eq!(
        core.message,
        "OpenAI accepts text and image content parts only"
    );
}

fn openai_text_response(text: &str) -> serde_json::Value {
    json!({
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
    })
}
