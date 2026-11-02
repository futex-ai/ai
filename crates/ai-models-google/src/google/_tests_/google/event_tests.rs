//! Public completion-event tests for Google generate-content streams.

use ai_interface::{ModelError, StructuredOutputSchema};
use ai_models_core::test_support::collect_model_events;
use serde_json::{Value, json};

use super::stream_support::custom_spec;
use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn emits_thought_and_assistant_fragments_with_part_boundary_parity() {
    let chunks = vec![
        json!({"candidates": [{"content": {"parts": [
            {"text": "hidden ", "thought": true}
        ]}}]}),
        json!({"candidates": [{"content": {"parts": [
            {"text": "reasoning", "thought": true}
        ]}}]}),
        json!({"candidates": [{"content": {"parts": [{"text": "First"}]}}]}),
        json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {"parts": [{"text": "Second"}, {"text": "Third"}]}
            }],
            "usageMetadata": {"promptTokenCount": 4, "candidatesTokenCount": 3}
        }),
    ];
    let stream = chunks.into_iter().map(stream_support::event).collect();
    let (http_client, _) = stream_support::recording_streaming_client(vec![stream]);
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
    let (response, events) = collect_model_events(&model, &simple_request()).await;
    let response = response.expect("stream completion should succeed");

    assert_eq!(
        events,
        vec![
            reasoning_delta(0, "hidden "),
            reasoning_delta(0, "reasoning"),
            assistant_delta(1, "First"),
            assistant_delta(1, "Second"),
            assistant_delta(2, "\nThird"),
            usage_event(4, 3, 7),
            finish_event(None),
        ]
    );
    assert_eq!(assistant_text(&events), response.text());
}

#[tokio::test]
async fn structured_completion_emits_candidate_text_and_validated_finish() {
    let chunks = vec![
        json!({"candidates": [{"content": {"parts": [
            {"text": "{\"summary\":"}
        ]}}]}),
        json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {"parts": [{"text": "\"Done\"}"}]}
            }]
        }),
    ];
    let stream = chunks.into_iter().map(stream_support::event).collect();
    let (http_client, _) = stream_support::recording_streaming_client(vec![stream]);
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
    let mut request = simple_request();
    request.response_schema = Some(summary_schema());

    let (response, events) = collect_model_events(&model, &request).await;
    let response = response.expect("structured completion should succeed");

    assert_eq!(response.structured_output, Some(json!({"summary": "Done"})));
    assert_eq!(
        events,
        vec![
            assistant_delta(0, "{\"summary\":"),
            assistant_delta(0, "\"Done\"}"),
            usage_event(0, 0, 0),
            finish_event(Some(json!({"summary": "Done"}))),
        ]
    );
}

#[tokio::test]
async fn emits_partial_candidate_text_before_an_interruption() {
    let stream = vec![
        stream_support::event(
            json!({"candidates": [{"content": {"parts": [{"text": "partial"}]}}]}),
        ),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = stream_support::recording_streaming_client(vec![stream]);
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
    let (result, events) = collect_model_events(&model, &simple_request()).await;
    let error = result.expect_err("interrupted completion should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(events, vec![assistant_delta(0, "partial")]);
}

fn assistant_text(events: &[Value]) -> String {
    events
        .iter()
        .filter_map(|event| event.get("TextDelta")?.get("delta")?.as_str())
        .collect()
}

fn assistant_delta(part_index: usize, delta: &str) -> Value {
    json!({"TextDelta": {"part_index": part_index, "delta": delta}})
}

fn reasoning_delta(part_index: usize, delta: &str) -> Value {
    json!({"ReasoningDelta": {"part_index": part_index, "delta": delta}})
}

fn usage_event(input: u64, output: u64, total: u64) -> Value {
    json!({"Usage": {"usage": {"input_tokens": input, "output_tokens": output,
        "cached_input_tokens": 0, "reasoning_tokens": 0, "total_tokens": total,
        "estimated_cost_microusd": 0}}})
}

fn finish_event(structured: Option<Value>) -> Value {
    let mut finish = json!({"finish_reason": "Stop", "provider": "google", "model_id": "gemini-3.6-flash",
        "catalog_model_id": "google/gemini-3.6-flash", "effort": "disabled"});
    if let Some(structured) = structured {
        finish["structured_output"] = structured;
    }
    json!({"Finish": finish})
}

fn summary_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {"summary": {"type": "string"}},
            "required": ["summary"]
        }),
    }
}
