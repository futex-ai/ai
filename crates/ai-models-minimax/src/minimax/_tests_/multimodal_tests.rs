//! MiniMax multimodal request tests.

use ai_interface::{ConversationContentPart, ConversationMessage, Model, ModelRequest};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{MiniMaxModel, support::recording_http_client};

#[tokio::test]
async fn serializes_ordered_text_and_image_parts_as_data_urls() {
    let (http_client, requests) = recording_http_client([JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "I see it."}
            }]
        }),
    }]);
    MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user_with_parts(
                "fallback should not be sent",
                vec![
                    ConversationContentPart::Text {
                        text: "Before".to_owned(),
                    },
                    ConversationContentPart::Image {
                        mime_type: "image/png".to_owned(),
                        data_base64: "aW1hZ2U=".to_owned(),
                    },
                    ConversationContentPart::Text {
                        text: "After".to_owned(),
                    },
                ],
            )],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("multimodal response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON body should be present");
    assert_eq!(
        body["messages"][1]["content"],
        json!([
            {"type": "text", "text": "Before"},
            {
                "type": "image_url",
                "image_url": {
                    "url": "data:image/png;base64,aW1hZ2U="
                }
            },
            {"type": "text", "text": "After"}
        ])
    );
}
