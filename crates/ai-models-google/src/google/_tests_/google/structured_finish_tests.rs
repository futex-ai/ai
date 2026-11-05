//! Tests for structured-output terminal finish handling.

use ai_interface::{FinishReason, Model, StructuredOutputSchema};
use serde_json::json;

use super::stream_support::custom_spec;
use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn structured_output_filter_preserves_filtered_finish_reason() {
    let response = parse_fixture(
        json!({
            "candidates": [{
                "finishReason": "SAFETY"
            }]
        }),
        Some(&status_schema()),
    )
    .await
    .expect("filtered structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert_eq!(response.structured_output, None);
    assert!(response.text().is_empty());
}

#[tokio::test]
async fn structured_output_truncation_preserves_truncated_finish_reason() {
    let response = parse_fixture(
        json!({
            "candidates": [{
                "finishReason": "MAX_TOKENS",
                "content": {
                    "parts": [{ "text": "{\"summary\":" }]
                }
            }]
        }),
        Some(&status_schema()),
    )
    .await
    .expect("truncated structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.text(), "{\"summary\":");
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "summary": {"type": "string"},
                "done": {"type": "boolean"}
            },
            "required": ["summary", "done"]
        }),
    }
}

async fn parse_fixture(
    body: serde_json::Value,
    schema: Option<&StructuredOutputSchema>,
) -> ai_interface::ModelResult<ai_interface::ModelResponse> {
    let (client, _) =
        stream_support::recording_streaming_client(vec![vec![stream_support::event(body)]]);
    let model = GoogleModel::new(client, custom_spec("gemini-2.5-pro"), "key");
    let mut request = simple_request();
    request.response_schema = schema.cloned();
    model.complete(&request).await
}
