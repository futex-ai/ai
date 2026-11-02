//! Signed replay parts do not change visible text or structured output.

use std::sync::{Arc, Mutex};

use ai_interface::{
    Model, ModelCompletionEvent, ModelCompletionEventSinkMock, StructuredOutputSchema,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::stream_support::custom_spec;
use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn signed_chunk_continues_previous_text_in_events_and_answer() {
    assert_signed_text(
        vec![
            json!([{"text": "Do"}]),
            json!([{"text": "ne", "thoughtSignature": "signed-tail"}]),
        ],
        &["Do", "ne"],
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
        &["Hello", " world"],
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
        &["The", " answer is 42.", "\nNext line"],
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
async fn schema_boolean_split_at_signed_boundary_parses_without_events() {
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

    let result = model
        .complete_with_events(&request, &Unimock::new(()))
        .await;

    assert!(result.is_ok(), "signed JSON must parse: {result:?}");
    let response = result.unwrap();
    assert_eq!(response.structured_output, Some(json!({"ok": true})));
    assert_eq!(response.assistant_message, "{\"ok\":true}");
}

async fn assert_signed_text(
    chunks: Vec<Value>,
    expected_deltas: &[&str],
    expected_answer: &str,
    replay_parts: Value,
) {
    let (client, _) = stream_support::recording_streaming_client(vec![text_stream(chunks)]);
    let model = GoogleModel::new(client, custom_spec("gemini-3.6-flash"), "key");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Unimock::new(
        ModelCompletionEventSinkMock::emit
            .each_call(matching!(_))
            .answers_arc({
                let events = events.clone();
                Arc::new(move |_, event| events.lock().unwrap().push(event))
            }),
    );

    let response = model
        .complete_with_events(&simple_request(), &sink)
        .await
        .unwrap();
    let expected_events = expected_deltas
        .iter()
        .map(|delta| ModelCompletionEvent::AssistantTextDelta {
            delta: (*delta).to_owned(),
        })
        .collect::<Vec<_>>();

    assert_eq!(
        (events.lock().unwrap().clone(), response.assistant_message),
        (expected_events, expected_answer.to_owned())
    );
    assert_eq!(
        serde_json::to_value(response.provider_context).unwrap(),
        json!([{"type": "google_assistant_message", "parts": replay_parts}])
    );
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
