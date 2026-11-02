//! Unhandled full Gemini replay stays useful after version two persistence.

use ai_interface::{ConversationMessage, Model, read_stored_message};
use serde_json::{Value, json};

use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn unhandled_legacy_google_state_survives_v2_storage_and_replays_one_full_message() {
    let original = json!([
        {"text": "private thought", "thought": true, "thoughtSignature": "thought-signature"},
        {"text": "  before  "},
        {"functionCall": {"name": "first"}, "thoughtSignature": "first-signature"},
        {"functionCall": {"id": "provider-second", "name": "second", "args": {"id": 7}}},
        {"text": "after", "thoughtSignature": "text-signature"},
        {"thoughtSignature": "signature-only"}
    ]);
    let portable_calls = json!([
        {"id": "runtime-first", "name": "first", "input": {}, "operation_id": "operation-first"},
        {"id": "runtime-second", "name": "second", "input": {"id": 7}},
        {"id": "runtime-extra", "name": "extra", "input": {}}
    ]);
    for (case, original, results) in [
        (
            "signed full message",
            original,
            json!([
                {"functionResponse": {"name": "first", "response": {"result": "one"}}},
                {"functionResponse": {"id": "provider-second", "name": "second", "response": {"result": "two"}}},
                {"functionResponse": {"id": "runtime-extra", "name": "extra", "response": {"result": "three"}}}
            ]),
        ),
        (
            "empty first full message",
            json!([]),
            json!([
                {"functionResponse": {"id": "runtime-first", "name": "first", "response": {"result": "one"}}},
                {"functionResponse": {"id": "runtime-second", "name": "second", "response": {"result": "two"}}},
                {"functionResponse": {"id": "runtime-extra", "name": "extra", "response": {"result": "three"}}}
            ]),
        ),
    ] {
        let message = read_stored_message(
            json!({
                "role": "assistant", "content": "portable fallback", "tool_calls": portable_calls,
                "provider_context": [
                    {"type": "google_assistant_message", "parts": original},
                    {"type": "google_assistant_message", "parts": [{"text": "ignored later full message"}]}
                ]
            }),
            &[],
        )
        .unwrap();
        let stored = serde_json::to_value(message).unwrap();
        assert_eq!(stored["format_version"], 2, "{case}");
        let message: ConversationMessage = serde_json::from_value(stored).unwrap();
        assert_eq!(message.text(), "portable fallback", "{case}");
        assert_eq!(
            serde_json::to_value(message.tool_calls()).unwrap(),
            portable_calls,
            "{case}"
        );
        assert_eq!(
            message
                .parts
                .iter()
                .find_map(|part| part.provider_state())
                .unwrap()
                .payload,
            json!({"parts": original}).as_object().unwrap().clone(),
            "{case}"
        );
        let actual = replay_request(message).await;
        assert_eq!(
            actual,
            json!({
                "systemInstruction": {"parts": [{"text": "system"}]},
                "contents": [
                    {"role": "user", "parts": [{"text": "hello"}]},
                    {"role": "model", "parts": original},
                    {"role": "user", "parts": results}
                ]
            }),
            "{case}"
        );
    }
}

async fn replay_request(message: ConversationMessage) -> Value {
    let (client, requests) =
        stream_support::recording_streaming_client(vec![vec![stream_support::event(
            json!({"candidates": [{"finishReason": "STOP"}]}),
        )]]);
    let model = GoogleModel::new(
        client,
        stream_support::custom_spec("gemini-3.6-flash"),
        "key",
    );
    let mut request = simple_request();
    request.messages.push(message);
    request.messages.extend([
        ConversationMessage::tool("one", "first", "runtime-first"),
        ConversationMessage::tool("two", "second", "runtime-second"),
        ConversationMessage::tool("three", "extra", "runtime-extra"),
    ]);
    model.complete(&request).await.unwrap();
    requests.lock().unwrap()[0]
        .body
        .as_ref()
        .unwrap()
        .as_json()
        .unwrap()
        .clone()
}
