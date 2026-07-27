//! MiniMax response-shape edge-case tests.

use ai_interface::{Model, ModelError, ModelUsage};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    MiniMaxModel,
    support::{recording_http_client, simple_request},
};

#[tokio::test]
async fn rejects_empty_choices_and_malformed_typed_fields() {
    let empty_error = complete(json!({"choices": []}))
        .await
        .expect_err("empty choices should fail");
    assert!(matches!(empty_error, ModelError::Provider { .. }));

    for malformed in [
        json!({"choices": "invalid"}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"message": {"reasoning_details": [{"index": "zero"}]}}]}),
    ] {
        let error = complete(malformed)
            .await
            .expect_err("malformed typed response should fail");
        assert!(matches!(error, ModelError::Internal { .. }));
    }
}

#[tokio::test]
async fn accepts_null_or_empty_content_and_absent_usage() {
    for content in [serde_json::Value::Null, json!("")] {
        let response = complete(json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": content}
            }]
        }))
        .await
        .expect("nullable content should normalize");

        assert_eq!(response.assistant_message, "");
        assert_eq!(response.usage, ModelUsage::default());
    }
}

async fn complete(
    body: serde_json::Value,
) -> std::result::Result<ai_interface::ModelResponse, ModelError> {
    let (http_client, _) = recording_http_client([JsonHttpResponse { status: 200, body }]);
    MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&simple_request())
        .await
}
