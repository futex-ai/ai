//! Ordered portable replay, opaque foreign state, and own-state validation.

use crate::openai::stream_support::error_context;
use ai_interface::{ConversationMessage, ConversationPart, ConversationRole, ProviderId};

use serde_json::{Value, json};

use super::{request_input::input_items, stream_support::state};

#[test]
fn replays_ordered_parts_and_ignores_malformed_foreign_state() {
    let mut reasoning_state = state(json!({"id":"rs-one","encrypted_content":"signed"}));
    reasoning_state.model = "another-openai-model".to_owned();
    let mut foreign = state(json!({"invalid":true}));
    foreign.provider = ProviderId::new("foreign");
    foreign.version = 77;
    let message = ConversationMessage::new(
        ConversationRole::Assistant,
        vec![
            ConversationPart::reasoning("").with_provider_state(reasoning_state),
            ConversationPart::text("first")
                .with_provider_state(state(json!({"phase":"commentary"}))),
            ConversationPart::ToolCall {
                call_id: "call-one".to_owned(),
                tool_name: "search".to_owned(),
                arguments: json!({"query":"rust"}),
                operation_id: None,
                provider_state: Some(foreign.clone()),
            },
            ConversationPart::reasoning("skip").with_provider_state(foreign.clone()),
            ConversationPart::text("last").with_provider_state(foreign),
        ],
    );
    assert_eq!(
        serde_json::to_value(
            input_items(&error_context("gpt-current"), "gpt-current", &[message]).unwrap()
        )
        .unwrap(),
        json!([
            {"type":"reasoning","id":"rs-one","summary":[],"encrypted_content":"signed"},
            {"role":"assistant","phase":"commentary","content":"first"},
            {"type":"function_call","call_id":"call-one","name":"search","arguments":"{\"query\":\"rust\"}"},
            {"role":"assistant","content":"last"}
        ])
    );
}

#[test]
fn rejects_invalid_own_state_on_every_part_before_mapping() {
    let parts = [
        (ConversationRole::User, ConversationPart::text("text")),
        (
            ConversationRole::User,
            ConversationPart::image("image/png", "img"),
        ),
        (
            ConversationRole::User,
            ConversationPart::video("video/mp4", "video"),
        ),
        (
            ConversationRole::Assistant,
            ConversationPart::text("answer"),
        ),
        (
            ConversationRole::Assistant,
            ConversationPart::reasoning("thought"),
        ),
        (
            ConversationRole::Assistant,
            ConversationPart::ToolCall {
                call_id: "a".to_owned(),
                tool_name: "search".to_owned(),
                arguments: json!({}),
                operation_id: None,
                provider_state: None,
            },
        ),
        (
            ConversationRole::Tool,
            ConversationPart::tool_result("result", "search", "a"),
        ),
    ];
    for (role, part) in parts {
        for (version, payload) in [
            (77, json!({"phase":"commentary"})),
            (1, json!({"phase":false})),
        ] {
            let mut own = state(payload);
            own.version = version;
            let result = input_items(
                &error_context("gpt-current"),
                "gpt-current",
                &[ConversationMessage::new(
                    role,
                    vec![part.clone().with_provider_state(own)],
                )],
            );
            assert!(
                matches!(result, Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidRequest && (error.core().provider == "openai" && error.core().model_id == "gpt-current" && error.core().message == if version == 77 { "unsupported provider state version" } else { "invalid provider state payload" })),
                "part: {part:?}"
            );
        }
    }
}

#[test]
fn phase_state_on_a_preservation_carrier_reaches_portable_text() {
    let mut foreign = state(json!({"bad":true}));
    foreign.provider = ProviderId::new("foreign");
    let message = ConversationMessage::new(
        ConversationRole::Assistant,
        vec![
            ConversationPart::reasoning("")
                .with_provider_state(state(json!({"phase":"commentary"}))),
            ConversationPart::text("visible").with_provider_state(foreign),
        ],
    );
    let input = input_items(&error_context("gpt-current"), "gpt-current", &[message]).unwrap();
    assert_eq!(
        serde_json::to_value(input).unwrap(),
        json!([{"role":"assistant","phase":"commentary","content":"visible"}])
    );
}

#[test]
fn rejects_parts_that_do_not_match_the_message_role() {
    let result = input_items(
        &error_context("gpt-current"),
        "gpt-current",
        &[ConversationMessage::new(
            ConversationRole::Tool,
            vec![ConversationPart::text("wrong")],
        )],
    );
    assert!(
        matches!(result, Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidRequest && (error.core().provider == "openai" && error.core().model_id == "gpt-current" && error.core().message == "conversation parts do not match the message role"))
    );
}

pub(super) fn mapped(messages: &[ConversationMessage]) -> Value {
    serde_json::to_value(
        input_items(&error_context("gpt-current"), "gpt-current", messages).unwrap(),
    )
    .unwrap()
}
