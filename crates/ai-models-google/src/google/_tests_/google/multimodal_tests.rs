use ai_interface::{ConversationContentPart, ConversationMessage, Model, ModelRequest, ToolCall};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{GoogleModel, recording_http_client};

#[tokio::test]
async fn serializes_image_context_message() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": { "parts": [{ "text": "Done" }] }
            }]
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

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
        .expect("Google response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let contents = &requests[0].body.as_ref().expect("body present")["contents"];
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
