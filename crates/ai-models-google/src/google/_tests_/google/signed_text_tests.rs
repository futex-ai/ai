//! Signed replay parts do not change visible text or structured output.

use ai_interface::StructuredOutputSchema;
use ai_models_core::test_support::collect_model_events;
use serde_json::{Value, json};

use super::stream_support::custom_spec;
use super::{GoogleModel, part_support::original_parts, simple_request, stream_support};

#[tokio::test]
async fn signed_chunk_continues_previous_text_in_events_and_answer() {
    assert_signed_text(
        vec![
            json!([{"text": "Do"}]),
            json!([{"text": "ne", "thoughtSignature": "signed-tail"}]),
        ],
        &[(0, "Do"), (0, "ne")],
        "Done",
        json!([
            {"text": "Do"},
            {"text": "ne", "thoughtSignature": "signed-tail"}
        ]),
    )
    .await;
}

#[tokio::test]
async fn text_after_signature_only_part_continues_in_events_and_answer() {
    assert_signed_text(
        vec![
            json!([{"text": "Hello"}]),
            json!([{"thoughtSignature": "signature-only"}]),
            json!([{"text": " world"}]),
        ],
        &[(0, "Hello"), (0, " world")],
        "Hello world",
        json!([
            {"text": "Hello"},
            {"thoughtSignature": "signature-only"},
            {"text": " world"}
        ]),
    )
    .await;
}

#[tokio::test]
async fn text_after_signed_first_chunk_continues_and_keeps_same_chunk_newline() {
    assert_signed_text(
        vec![
            json!([{"text": "The", "thoughtSignature": "signed-start"}]),
            json!([
                {"text": " answer is 42."},
                {"text": "Next line", "thoughtSignature": "signed-line"}
            ]),
        ],
        &[(0, "The"), (0, " answer is 42."), (1, "\nNext line")],
        "The answer is 42.\nNext line",
        json!([
            {"text": "The", "thoughtSignature": "signed-start"},
            {"text": " answer is 42."},
            {"text": "Next line", "thoughtSignature": "signed-line"}
        ]),
    )
    .await;
}

#[tokio::test]
async fn schema_boolean_split_at_signed_boundary_emits_text_and_parses() {
    let chunks = vec![
        json!([{"text": "{\"ok\":tr"}]),
        json!([{"text": "ue}", "thoughtSignature": "signed-json-tail"}]),
    ];
    let (client, _) = stream_support::recording_streaming_client(vec![text_stream(chunks)]);
    let model = GoogleModel::new(client, custom_spec("gemini-3.6-flash"), "key");
    let mut request = simple_request();
    request.response_schema = Some(StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean"}},
            "required": ["ok"]
        }),
    });

    let (result, events) = collect_model_events(&model, &request).await;

    assert!(result.is_ok(), "signed JSON must parse: {result:?}");
    let response = result.unwrap();
    assert_eq!(response.structured_output, Some(json!({"ok": true})));
    assert_eq!(response.text(), "{\"ok\":true}");
    assert_eq!(
        text_events(events),
        vec![
            json!({"TextDelta": {"part_index": 0, "delta": "{\"ok\":tr"}}),
            json!({"TextDelta": {"part_index": 0, "delta": "ue}"}}),
        ]
    );
}

async fn assert_signed_text(
    chunks: Vec<Value>,
    expected_deltas: &[(usize, &str)],
    expected_answer: &str,
    replay_parts: Value,
) {
    let (client, _) = stream_support::recording_streaming_client(vec![text_stream(chunks)]);
    let model = GoogleModel::new(client, custom_spec("gemini-3.6-flash"), "key");
    let (response, events) = collect_model_events(&model, &simple_request()).await;
    let response = response.expect("signed stream should fold");
    let expected_events = expected_deltas
        .iter()
        .map(|(part_index, delta)| json!({"TextDelta": {"part_index": part_index, "delta": delta}}))
        .collect::<Vec<_>>();

    assert_eq!(
        (text_events(events), response.text()),
        (expected_events, expected_answer.to_owned())
    );
    assert_eq!(original_parts(&response), replay_parts);
}

fn text_stream(chunks: Vec<Value>) -> Vec<stream_support::StreamItem> {
    let mut stream = chunks
        .into_iter()
        .map(|parts| stream_support::event(json!({"candidates": [{"content": {"parts": parts}}]})))
        .collect::<Vec<_>>();
    stream.push(stream_support::event(
        json!({"candidates": [{"finishReason": "STOP"}]}),
    ));
    stream
}

fn text_events(events: Vec<Value>) -> Vec<Value> {
    events
        .into_iter()
        .filter(|event| {
            event
                .get("TextDelta")
                .and_then(|delta| delta.get("delta"))
                .and_then(Value::as_str)
                .is_some_and(|text| !text.is_empty())
        })
        .collect()
}
