use ai_interface::{ConversationMessage, ConversationPart, Model, ModelRequest, ToolCall};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::stream_support::custom_spec;
use super::{GoogleModel, recording_http_client};

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": { "parts": [{ "text": "Done" }] }
            }]
        }),
    });
    let model = GoogleModel::new(http_client, custom_spec("gemini-2.5-pro"), "google-key");

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
        .expect("Google response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let contents = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["contents"];
    assert_eq!(
        contents[1]["parts"][0]["functionResponse"]["id"],
        "call-view"
    );
    assert_eq!(
        contents[1]["parts"][1]["text"],
        "Visual context is available."
    );
    assert_eq!(
        contents[1]["parts"][2]["inlineData"]["mimeType"],
        "image/png"
    );
    assert_eq!(contents[1]["parts"][2]["inlineData"]["data"], "abc123");
}

#[tokio::test]
async fn serializes_video_context_message_as_inline_data() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": { "parts": [{ "text": "Watched." }] }
            }]
        }),
    });
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "google-key");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user_with_parts(vec![
                ConversationPart::Text {
                    text: "What happens in this clip?".to_owned(),
                    provider_state: None,
                },
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
        .expect("Google response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let contents = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["contents"];
    assert_eq!(contents[0]["role"], "user");
    assert_eq!(
        contents[0]["parts"][0]["text"],
        "What happens in this clip?"
    );
    assert_eq!(
        contents[0]["parts"][1]["inlineData"]["mimeType"],
        "video/mp4"
    );
    assert_eq!(contents[0]["parts"][1]["inlineData"]["data"], "dmlkZW8=");
}
