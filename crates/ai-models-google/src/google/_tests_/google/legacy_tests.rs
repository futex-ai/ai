//! Stored Gemini content keeps its original order and runtime call metadata.

use ai_interface::{ConversationMessage, ConversationRole, Model, ToolCall, read_stored_message};
use serde_json::json;

use crate::GoogleLegacyProviderConverter;

use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn converted_google_content_preserves_order_signatures_and_missing_call_fields() {
    let original = json!([
        {"text": "before", "thoughtSignature": "text-signature"},
        {"functionCall": {"name": "status"}},
        {"text": "  ", "thought": false},
        {"text": "private thought", "thought": true},
        {"text": "after"},
        {"thoughtSignature": "signature-only"}
    ]);
    let converted = read_stored_message(
        json!({
            "role": "assistant", "name": "helper", "content": "normalized fallback",
            "tool_calls": [{
                "id": "runtime-call", "name": "status", "input": {}, "operation_id": "operation-7"
            }],
            "provider_context": [{"type": "google_assistant_message", "parts": original}]
        }),
        &[&GoogleLegacyProviderConverter],
    )
    .unwrap();
    assert_eq!(
        (converted.role, converted.name.as_deref()),
        (ConversationRole::Assistant, Some("helper"))
    );
    assert_eq!(
        converted.tool_calls(),
        vec![ToolCall {
            id: "runtime-call".to_owned(),
            name: "status".to_owned(),
            input: json!({}),
            operation_id: Some("operation-7".to_owned()),
        }]
    );
    assert_eq!(
        serde_json::to_value(&converted).unwrap()["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| part["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "text",
            "tool_call",
            "text",
            "reasoning",
            "text",
            "reasoning"
        ]
    );
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
    request.messages.push(converted);
    request
        .messages
        .push(ConversationMessage::tool("ready", "status", "runtime-call"));

    model.complete(&request).await.unwrap();

    let requests = requests.lock().unwrap();
    let contents = &requests[0].body.as_ref().unwrap()["contents"];
    assert_eq!(contents[1]["parts"], original);
    assert_eq!(
        contents[2]["parts"],
        json!([
            {"functionResponse": {"name": "status", "response": {"result": "ready"}}}
        ])
    );
}
