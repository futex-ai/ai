//! Stored parts, role validation, state validation, and redacted observer copies.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    ConversationMessage, ConversationPart, ConversationRole, FinishReason, ModelError,
    ModelRequest, ModelResponse, ProviderId, ProviderState, StoredMessageError, ToolCall,
    read_stored_message,
};

pub(super) fn state(token: &str) -> ProviderState {
    ProviderState {
        provider: ProviderId::new("sample"),
        model: "model-one".to_owned(),
        version: 1,
        payload: json!({"token": token}).as_object().unwrap().clone(),
    }
}

pub(super) fn call() -> ToolCall {
    ToolCall {
        id: "call-one".to_owned(),
        name: "search".to_owned(),
        input: json!({"query": "rust"}),
        operation_id: Some("operation-one".to_owned()),
    }
}

#[test]
fn log_copies_remove_state_and_reasoning_without_changing_sources() {
    let parts = vec![
        ConversationPart::text("visible"),
        ConversationPart::image("image/png", "aW1n"),
        ConversationPart::video("video/mp4", "dmlk"),
        ConversationPart::from_tool_call(call()),
        ConversationPart::tool_result("result", "search", "call-one"),
        ConversationPart::reasoning("private thought"),
    ]
    .into_iter()
    .map(|part| part.with_provider_state(state("secret")))
    .collect::<Vec<_>>();
    let message = ConversationMessage {
        role: ConversationRole::Assistant,
        parts: parts.clone(),
        name: Some("helper".to_owned()),
    };
    let mut expected = message.clone();
    expected.parts = vec![
        ConversationPart::text("visible"),
        ConversationPart::image("image/png", "aW1n"),
        ConversationPart::video("video/mp4", "dmlk"),
        ConversationPart::from_tool_call(call()),
        ConversationPart::tool_result("result", "search", "call-one"),
        ConversationPart::reasoning(""),
    ];
    assert_eq!(message.log_copy(), expected);
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![message.clone()],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    let response = ModelResponse {
        provider: "sample".to_owned(),
        model_id: "model-one".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: parts.clone(),
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: Default::default(),
        attempts: Vec::new(),
    };
    assert_eq!(request.log_copy().messages, vec![expected.clone()]);
    assert_eq!(response.log_copy().parts, expected.parts);
    assert_eq!(message.parts, parts);
    assert_eq!(request.messages, vec![message]);
    assert_eq!(response.parts, parts);
    assert_eq!(response.to_assistant_message().parts, parts);
}

#[test]
fn stored_message_writes_v2_and_round_trips_ordered_parts() {
    let message = ConversationMessage {
        role: ConversationRole::Assistant,
        parts: vec![
            ConversationPart::reasoning("").with_provider_state(state("signed")),
            ConversationPart::text("answer"),
            ConversationPart::from_tool_call(call()),
        ],
        name: Some("helper".to_owned()),
    };
    let value = serde_json::to_value(&message).unwrap();
    assert_eq!(
        value,
        json!({"format_version":2,"role":"assistant","name":"helper","parts":[
            {"type":"reasoning","text":"","provider_state":{"provider":"sample","model":"model-one","version":1,"payload":{"token":"signed"}}},
            {"type":"text","text":"answer"},
            {"type":"tool_call","call_id":"call-one","tool_name":"search","arguments":{"query":"rust"},"operation_id":"operation-one"}
        ]})
    );
    assert_eq!(read_stored_message(value.clone(), &[]).unwrap(), message);
    insta::assert_json_snapshot!("stored_v2_parts", value);
}

#[test]
fn stored_reader_converts_v1_roles_without_losing_portable_fields() {
    let inputs = [
        (
            json!({"role":"user","content":"ignored","name":"caller","content_parts":[{"type":"text","text":"first"},{"type":"video","mime_type":"video/mp4","data_base64":"dmlk"}]}),
            ConversationMessage {
                role: ConversationRole::User,
                parts: vec![
                    ConversationPart::text("first"),
                    ConversationPart::video("video/mp4", "dmlk"),
                ],
                name: Some("caller".to_owned()),
            },
        ),
        (
            json!({"role":"user","content":"fallback"}),
            ConversationMessage::user("fallback"),
        ),
        (
            json!({"role":"user","content":""}),
            ConversationMessage::user_with_parts(Vec::new()),
        ),
        (
            json!({"role":"assistant","content":"answer","name":"helper","content_parts":[{"type":"text","text":"drop"}],"tool_calls":[{"id":"call-one","name":"search","input":{"query":"rust"},"operation_id":"operation-one"}]}),
            ConversationMessage {
                role: ConversationRole::Assistant,
                parts: vec![
                    ConversationPart::text("answer"),
                    ConversationPart::from_tool_call(call()),
                ],
                name: Some("helper".to_owned()),
            },
        ),
        (
            json!({"role":"tool","content":"result","name":"search","tool_call_id":"call-one","content_parts":[{"type":"text","text":"drop"}]}),
            ConversationMessage::tool("result", "search", "call-one"),
        ),
    ];
    for (input, expected) in inputs {
        assert_eq!(read_stored_message(input, &[]).unwrap(), expected);
    }
}

#[test]
fn stored_reader_reports_unsupported_format_versions() {
    let error = read_stored_message(json!({"format_version":7,"role":"user","parts":[]}), &[]);
    assert!(matches!(
        error,
        Err(StoredMessageError::UnsupportedFormatVersion { version: 7 })
    ));
}

#[test]
fn message_role_table_rejects_invalid_parts_before_transport() {
    let invalid = [
        ConversationMessage::new(
            ConversationRole::User,
            vec![ConversationPart::reasoning("private")],
        ),
        ConversationMessage::new(
            ConversationRole::Assistant,
            vec![ConversationPart::image("image/png", "raw")],
        ),
        ConversationMessage::new(ConversationRole::Tool, Vec::new()),
        ConversationMessage::new(
            ConversationRole::Tool,
            vec![
                ConversationPart::tool_result("one", "search", "a"),
                ConversationPart::tool_result("two", "search", "b"),
            ],
        ),
    ];
    for message in invalid {
        assert!(
            matches!(message.validate("sample", "model-one"), Err(ModelError::Provider { provider, model_id, message }) if provider == "sample" && model_id == "model-one" && message == "conversation parts do not match the message role")
        );
    }
    for message in [
        ConversationMessage::user_with_parts(Vec::new()),
        ConversationMessage::assistant("", Vec::new()),
        ConversationMessage::tool("result", "search", "a"),
    ] {
        assert_eq!(message.validate("sample", "model-one").ok(), Some(()));
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Payload {
    token: String,
}

#[test]
fn own_state_validation_rejects_unknown_versions_and_invalid_payloads() {
    assert_eq!(
        state("signed")
            .decode::<Payload>(&ProviderId::new("sample"), "model-two", 1)
            .unwrap(),
        Some(Payload {
            token: "signed".to_owned()
        })
    );
    let mut foreign = state("signed");
    foreign.provider = ProviderId::new("other");
    foreign.version = 99;
    foreign.payload.insert("token".to_owned(), Value::Null);
    assert_eq!(
        foreign
            .decode::<Payload>(&ProviderId::new("sample"), "model-two", 1)
            .unwrap(),
        None
    );
    for (version, payload) in [
        (0, json!({"token":"signed"})),
        (9, json!({"token":"signed"})),
        (1, json!({"token":null})),
    ] {
        let own = ProviderState {
            version,
            payload: payload.as_object().unwrap().clone(),
            ..state("signed")
        };
        assert!(
            matches!(own.decode::<Payload>(&ProviderId::new("sample"), "model-two", 1), Err(ModelError::Provider { provider, model_id, .. }) if provider == "sample" && model_id == "model-two")
        );
    }
}
