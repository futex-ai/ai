//! Tests for structured-output terminal finish handling.

use ai_interface::{Effort, FinishReason, StructuredOutputSchema};
use serde_json::json;

use super::stream_support::response_from_fixture;

#[tokio::test]
async fn structured_output_refusal_preserves_filtered_finish_reason() {
    let response = response_from_fixture(
        "gpt-5.5",
        "gpt-5.5",
        Effort::Medium,
        json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "refusal",
                    "refusal": "I cannot provide that."
                }]
            }]
        }),
        Some(&status_schema()),
    )
    .await
    .expect("filtered structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.text(), "I cannot provide that.");
}

#[tokio::test]
async fn structured_output_truncation_preserves_truncated_finish_reason() {
    let response = response_from_fixture(
        "gpt-5.5",
        "gpt-5.5",
        Effort::Medium,
        json!({
            "status": "incomplete",
            "incomplete_details": { "reason": "max_output_tokens" },
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "{\"summary\":"
                }]
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
