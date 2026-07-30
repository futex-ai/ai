//! DeepSeek typed response-shape edge-case tests.

use ai_interface::{Model, ModelError};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    DeepSeekModel,
    test_support::{recording_http_client, simple_request},
};

#[tokio::test]
async fn missing_or_empty_choices_are_provider_failures() {
    for body in [json!({}), json!({"choices": []})] {
        let error = complete(body)
            .await
            .expect_err("missing first choice should fail");
        assert!(matches!(error, ModelError::Provider { .. }));
    }
}

#[tokio::test]
async fn malformed_typed_response_fields_retain_internal_sources() {
    for body in [
        json!({"choices": "invalid"}),
        json!({"choices": [{}]}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"finish_reason": 7, "message": {"content": "Done"}}]}),
        json!({
            "choices": [{"message": {"content": "Done"}}],
            "usage": {"prompt_tokens": "many"}
        }),
        json!({
            "choices": [{"message": {"content": "Done"}}],
            "usage": {
                "completion_tokens_details": {"reasoning_tokens": "many"}
            }
        }),
    ] {
        let error = complete(body)
            .await
            .expect_err("malformed typed response should fail");
        assert!(matches!(error, ModelError::Internal { .. }));
    }
}

async fn complete(
    body: serde_json::Value,
) -> std::result::Result<ai_interface::ModelResponse, ModelError> {
    let (http_client, _) = recording_http_client(JsonHttpResponse { status: 200, body });
    DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
}
