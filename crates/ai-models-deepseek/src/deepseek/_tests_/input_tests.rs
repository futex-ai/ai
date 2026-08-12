//! DeepSeek local input validation tests.

use std::sync::Arc;

use ai_interface::{ConversationContentPart, ConversationMessage, Model, ModelError, ModelRequest};
use json_http::JsonHttpAuth;
use unimock::Unimock;

use super::{DeepSeekModel, test_support::unused_http_client};

#[tokio::test]
async fn rejects_every_non_empty_content_parts_value_before_external_calls() {
    let cases = [
        ConversationContentPart::Text {
            text: "typed text".to_owned(),
        },
        ConversationContentPart::Image {
            mime_type: "image/png".to_owned(),
            data_base64: "abc123".to_owned(),
        },
    ];

    for part in cases {
        let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(()));
        let model = DeepSeekModel::with_auth(unused_http_client(), auth);
        let error = model
            .complete(&ModelRequest {
                system_prompt: "system".to_owned(),
                messages: vec![ConversationMessage::user_with_parts("fallback", vec![part])],
                tools: Vec::new(),
                response_schema: None,
                controls: Default::default(),
            })
            .await
            .expect_err("typed content should be rejected");

        assert!(matches!(error, ModelError::Provider { .. }));
    }
}
