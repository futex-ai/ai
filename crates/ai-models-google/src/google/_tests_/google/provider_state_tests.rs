//! Own replay state is decoded before transport and foreign state stays portable.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, Model, ProviderId, ProviderState,
};
use json_http::TransportBackedJsonHttpClient;
use serde_json::json;
use unimock::Unimock;

use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn replays_own_state_from_another_google_model_and_skips_foreign_reasoning() {
    let (client, requests) =
        stream_support::recording_streaming_client(vec![vec![stream_support::event(
            json!({"candidates": [{"finishReason": "STOP"}]}),
        )]]);
    let model = GoogleModel::new(
        client,
        stream_support::custom_spec("gemini-3.6-flash"),
        "key",
    );
    let own = state(
        "google",
        1,
        json!({"scope": "part", "parts": [{"text": "saved thought", "thought": true}]}),
    );
    let foreign = state("anthropic", 99, json!({"unreadable": true}));
    let mut request = simple_request();
    request.messages.push(ConversationMessage::new(
        ConversationRole::Assistant,
        vec![
            ConversationPart::reasoning("saved thought").with_provider_state(own),
            ConversationPart::reasoning("foreign private").with_provider_state(foreign.clone()),
            ConversationPart::reasoning("no state"),
            ConversationPart::text("portable").with_provider_state(foreign.clone()),
            ConversationPart::ToolCall {
                call_id: "call-7".to_owned(),
                tool_name: "lookup".to_owned(),
                arguments: json!({"id": 7}),
                operation_id: None,
                provider_state: Some(foreign),
            },
        ],
    ));

    model.complete(&request).await.unwrap();

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            requests.lock().unwrap()[0]
                .body
                .as_ref()
                .unwrap()
                .as_json_bytes()
                .expect("JSON request body")
        )
        .expect("valid JSON request body")["contents"][1]["parts"],
        json!([
            {"text": "saved thought", "thought": true},
            {"text": "portable"},
            {"functionCall": {"id": "call-7", "name": "lookup", "args": {"id": 7}}}
        ])
    );
}

#[tokio::test]
async fn rejects_unknown_or_invalid_own_state_and_invalid_roles_before_transport() {
    for (parts, expected) in [
        (
            vec![ConversationPart::text("x").with_provider_state(state(
                "google",
                2,
                json!({"parts": []}),
            ))],
            "Google provider state has unsupported version 2",
        ),
        (
            vec![ConversationPart::reasoning("x").with_provider_state(state(
                "google",
                1,
                json!({"parts": "invalid"}),
            ))],
            "Google provider state has an invalid payload",
        ),
        (
            vec![
                ConversationPart::reasoning("").with_provider_state(state(
                    "google",
                    1,
                    json!({"parts": []}),
                )),
                ConversationPart::text("x").with_provider_state(state(
                    "google",
                    2,
                    json!({"scope": "part", "parts": []}),
                )),
            ],
            "Google provider state has unsupported version 2",
        ),
    ] {
        let model = no_call_model();
        let mut request = simple_request();
        request
            .messages
            .push(ConversationMessage::new(ConversationRole::Assistant, parts));
        let actual = model.complete(&request).await.err().map(|error| {
            let core = error.core();
            (
                core.kind,
                core.provider.clone(),
                core.model_id.clone(),
                core.message.clone(),
            )
        });
        assert_eq!(
            actual,
            Some((
                ai_interface::ErrorKind::InvalidRequest,
                "google".to_owned(),
                "gemini-3.6-flash".to_owned(),
                expected.to_owned()
            ))
        );
    }
    let model = no_call_model();
    let mut request = simple_request();
    request.messages = vec![ConversationMessage::new(
        ConversationRole::User,
        vec![ConversationPart::reasoning("invalid user thought")],
    )];
    assert!(
        matches!(model.complete(&request).await, Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidRequest)
    );
}

fn no_call_model() -> GoogleModel {
    GoogleModel::new(
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
            (),
        )))),
        stream_support::custom_spec("gemini-3.6-flash"),
        "key",
    )
}

fn state(provider: &'static str, version: u32, payload: serde_json::Value) -> ProviderState {
    ProviderState {
        provider: ProviderId::new(provider),
        model: "another-google-model".to_owned(),
        version,
        payload: payload.as_object().unwrap().clone(),
    }
}
