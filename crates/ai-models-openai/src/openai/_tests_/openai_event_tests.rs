//! Public completion-event tests for OpenAI Responses streams.

use ai_interface::{ConversationMessage, ModelRequest, StructuredOutputSchema};
use ai_models_core::test_support::collect_model_events;
use serde_json::{Value, json};

use crate::openai::stream_support::custom_spec;
use crate::openai::stream_support::{event, recording_streaming_client, terminal_event};

use super::OpenAiModel;

#[tokio::test]
async fn emits_output_and_reasoning_deltas_in_provider_order_with_parity() {
    let events = vec![
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "Hel"}),
        ),
        event(
            "response.reasoning_summary_text.delta",
            json!({
                "type": "response.reasoning_summary_text.delta",
                "delta": "think"
            }),
        ),
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "lo"}),
        ),
        event(
            "response.reasoning_summary_text.delta",
            json!({
                "type": "response.reasoning_summary_text.delta",
                "delta": "ing"
            }),
        ),
        terminal_event("response.completed", text_body("Hello")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");
    let (response, events) = collect_model_events(&model, &simple_request()).await;
    let response = response.expect("stream completion should succeed");

    assert_eq!(
        events,
        vec![
            assistant_delta(0, "Hel"),
            reasoning_delta(1, "think"),
            assistant_delta(0, "lo"),
            reasoning_delta(1, "ing"),
            usage_event(),
            finish_event(None),
        ]
    );
    assert_eq!(assistant_text(&events), response.text());
}

#[tokio::test]
async fn structured_completion_emits_output_deltas_and_validated_finish() {
    let events = vec![
        event(
            "response.output_text.delta",
            json!({
                "type": "response.output_text.delta",
                "delta": "{\"summary\":\"Done\"}"
            }),
        ),
        terminal_event("response.completed", text_body("{\"summary\":\"Done\"}")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");
    let mut request = simple_request();
    request.response_schema = Some(summary_schema());

    let (response, events) = collect_model_events(&model, &request).await;
    let response = response.expect("structured stream should succeed");

    assert_eq!(response.structured_output, Some(json!({"summary": "Done"})));
    assert_eq!(
        events,
        vec![
            assistant_delta(0, "{\"summary\":\"Done\"}"),
            usage_event(),
            finish_event(Some(json!({"summary": "Done"}))),
        ]
    );
}

#[tokio::test]
async fn emits_partial_output_before_an_interruption() {
    let events = vec![
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "partial"}),
        ),
        Err(json_http::Error::transport("reset")),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");
    let (result, events) = collect_model_events(&model, &simple_request()).await;
    let error = result.expect_err("interrupted completion should fail");

    assert!(
        matches!(error, ref error if error.core().kind == ai_interface::ErrorKind::Interrupted)
    );
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

fn usage_event() -> Value {
    json!({"Usage": {"usage": {"input_tokens": 4, "output_tokens": 2,
        "cached_input_tokens": 0, "reasoning_tokens": 0, "total_tokens": 6,
        "estimated_cost_microusd": 0}}})
}

fn finish_event(structured: Option<Value>) -> Value {
    let mut finish = json!({"finish_reason": "Stop", "provider": "openai", "model_id": "gpt-5.5",
        "catalog_model_id": "openai/gpt-5.5"});
    if let Some(structured) = structured {
        finish["structured_output"] = structured;
    }
    json!({"Finish": finish})
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn text_body(text: &str) -> Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": text}]
        }],
        "usage": {"input_tokens": 4, "output_tokens": 2, "total_tokens": 6}
    })
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
