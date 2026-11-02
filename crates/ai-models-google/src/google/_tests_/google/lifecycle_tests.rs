//! Indexed tool calls, terminal metadata, and failed stream closure.

use ai_interface::{Model, ModelError, StructuredOutputSchema};
use serde_json::json;

use super::stream_support::custom_spec;
use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn emits_complete_function_call_before_last_usage_and_finish() {
    let chunks = vec![
        stream_support::event(json!({"usageMetadata": {"promptTokenCount": 1}})),
        stream_support::event(json!({
            "candidates": [{"finishReason": "STOP", "content": {"parts": [
                {"text": "  "}, {"functionCall": {"id": "call_1", "name": "status"}}
            ]}}, {"content": {"parts": [{"text": "ignore candidate one"}]}}],
            "usageMetadata": {
                "promptTokenCount": 10, "candidatesTokenCount": 3, "cachedContentTokenCount": 2,
                "thoughtsTokenCount": 4, "totalTokenCount": 17
            }
        })),
    ];
    let (client, _) = stream_support::recording_streaming_client(vec![chunks]);
    let model = GoogleModel::new(client, custom_spec("gemini-3.6-flash"), "key");
    let mut stream = model
        .stream(&simple_request())
        .await
        .expect("stream should open");
    let mut actual = Vec::new();
    while let Some(event) = stream.next().await.expect("event should succeed") {
        actual.push(serde_json::to_value(event).expect("event should serialize"));
    }

    assert_eq!(
        actual,
        vec![
            json!({"ToolCallStart": {"part_index": 0, "id": "call_1", "name": "status"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 0, "delta": "{}"}}),
            json!({"ToolCallEnd": {"part_index": 0}}),
            json!({"Usage": {"usage": {
                "input_tokens": 8, "output_tokens": 3, "cached_input_tokens": 2,
                "reasoning_tokens": 4, "total_tokens": 17, "estimated_cost_microusd": 0
            }}}),
            json!({"Finish": {
                "finish_reason": "ToolCalls", "provider": "google", "model_id": "gemini-3.6-flash",
                "catalog_model_id": "google/gemini-3.6-flash", "effort": "disabled"
            }}),
        ]
    );
    assert!(
        stream
            .next()
            .await
            .expect("closed stream should stay closed")
            .is_none()
    );
}

#[tokio::test]
async fn invalid_schema_fails_before_usage_and_then_closes() {
    let chunks = vec![stream_support::event(json!({
        "candidates": [{"finishReason": "STOP", "content": {"parts": [{"text": "{}"}]}}]
    }))];
    let (client, _) = stream_support::recording_streaming_client(vec![chunks]);
    let model = GoogleModel::new(client, custom_spec("gemini-3.6-flash"), "key");
    let mut request = simple_request();
    request.response_schema = Some(StructuredOutputSchema {
        name: "summary".to_owned(),
        schema: json!({
            "type": "object", "properties": {"summary": {"type": "string"}}, "required": ["summary"]
        }),
    });
    let mut stream = model.stream(&request).await.expect("stream should open");
    let text = stream.next().await.expect("text should arrive");
    assert_eq!(
        serde_json::to_value(text).expect("event should serialize"),
        json!({"TextDelta": {"part_index": 0, "delta": "{}"}})
    );
    let invalid = stream
        .next()
        .await
        .expect_err("invalid schema must precede usage");
    assert!(matches!(invalid, ModelError::Provider { .. }));
    assert!(
        stream
            .next()
            .await
            .expect("failed stream should stay closed")
            .is_none()
    );
}
