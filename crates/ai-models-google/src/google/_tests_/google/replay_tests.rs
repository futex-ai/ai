//! Signed Gemini content survives streams, storage, and continuation replay.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Model};
use ai_models_core::ThinkingLevel;
use json_http::{DynJsonHttpClient, StaticHeaderAuth};
use serde_json::json;

use crate::{GEMINI_3_6_FLASH, GEMINI_3_6_FLASH_THINKING_HIGH, GoogleModel};

use super::{
    simple_request,
    stream_support::{event, recording_streaming_client},
};

#[tokio::test]
async fn signed_google_parts_replay_without_merging_or_synthetic_ids() {
    let parts = json!([
        {"text": "private thought", "thought": true, "thoughtSignature": "thought-signature"},
        {"text": "  before  ", "thought": false},
        {"functionCall": {"name": "lookup", "args": {"id": 7}}, "thoughtSignature": "call-signature"},
        {"functionCall": {"id": "call-2", "name": "no_args"}},
        {"text": "after", "thoughtSignature": "text-signature"},
        {"text": "", "thoughtSignature": "empty-text-signature"},
        {"thoughtSignature": "signature-only"}
    ]);
    let (client, requests) = recording_streaming_client(vec![
        vec![event(
            json!({"candidates": [{"content": {"parts": parts}, "finishReason": "STOP"}]}),
        )],
        vec![event(
            json!({"candidates": [{"content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"}]}),
        )],
    ]);
    let model = thinking_model(client);
    let mut request = simple_request();
    let response = model.complete(&request).await.unwrap();
    assert_eq!(
        serde_json::to_value(&response.provider_context).unwrap(),
        json!([
            {"type": "google_assistant_message", "parts": parts}
        ])
    );
    assert_eq!(response.assistant_message, "  before  \nafter");
    assert_eq!(response.tool_calls.len(), 2);
    let assistant = ConversationMessage::assistant_with_provider_context(
        "normalized fallback",
        Vec::new(),
        response.provider_context,
    );
    let stored = serde_json::from_value(serde_json::to_value(assistant).unwrap()).unwrap();
    request.messages.push(stored);
    request
        .messages
        .push(ConversationMessage::assistant("next message", Vec::new()));
    request.messages.push(ConversationMessage::tool(
        "7",
        "lookup",
        &response.tool_calls[0].id,
    ));
    model.complete(&request).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests[1].body.as_ref().unwrap()["contents"][1]["parts"],
        parts
    );
}

#[tokio::test]
async fn streamed_google_signature_chunks_keep_their_own_parts() {
    let (client, requests) = recording_streaming_client(vec![
        vec![
            event(json!({"candidates": [{"content": {"parts": [{"text": "Do"}]}}]})),
            event(json!({"candidates": [{"content": {"parts": [{"text": "ne"}]}}]})),
            event(
                json!({"candidates": [{"content": {"parts": [{"text": "", "thoughtSignature": "tail"}]}}]}),
            ),
            event(
                json!({"candidates": [{"content": {"parts": [{"text": "separate", "thoughtSignature": "other"}]}, "finishReason": "STOP"}]}),
            ),
        ],
        vec![event(
            json!({"candidates": [{"content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"}]}),
        )],
    ]);
    let model = thinking_model(client);
    let mut request = simple_request();
    let response = model.complete(&request).await.unwrap();
    let parts = json!([
        {"text": "Done"}, {"text": "", "thoughtSignature": "tail"},
        {"text": "separate", "thoughtSignature": "other"}
    ]);
    assert_eq!(
        serde_json::to_value(&response.provider_context).unwrap(),
        json!([
            {"type": "google_assistant_message", "parts": parts}
        ])
    );
    request
        .messages
        .push(ConversationMessage::assistant_with_provider_context(
            response.assistant_message,
            response.tool_calls,
            response.provider_context,
        ));
    model.complete(&request).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests[1].body.as_ref().unwrap()["contents"][1]["parts"],
        parts
    );
}

fn thinking_model(client: DynJsonHttpClient) -> GoogleModel {
    GoogleModel::with_catalog_auth(
        client,
        GEMINI_3_6_FLASH_THINKING_HIGH,
        GEMINI_3_6_FLASH,
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::new(Default::default())),
    )
}
