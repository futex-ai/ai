//! Judgment input validation and stored conversation serde tests.

use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    ConversationMessage, ErrorKind, JudgmentError, JudgmentInput, JudgmentQuestion,
    JudgmentRequest, JudgmentResult, ToolCall,
};

#[test]
fn conversation_input_uses_stored_messages_in_order() {
    let input = JudgmentInput::Conversation {
        system: Some("  Use the conversation  ".to_owned()),
        messages: vec![
            ConversationMessage::user("My invoice is due."),
            ConversationMessage::assistant("I can check that.", Vec::new()),
        ],
    };
    let expected = json!({
        "system": "  Use the conversation  ",
        "messages": [
            {"format_version": 2, "role": "user", "parts": [{"type": "text", "text": "My invoice is due."}]},
            {"format_version": 2, "role": "assistant", "parts": [{"type": "text", "text": "I can check that."}]}
        ]
    });

    assert_eq!(serde_json::to_value(&input).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<JudgmentInput>(expected).unwrap(),
        input
    );
}

#[test]
fn blank_system_text_is_omitted_and_decodes_as_absent() {
    let input = JudgmentInput::Conversation {
        system: Some(" \t\n".to_owned()),
        messages: Vec::new(),
    };
    let absent = JudgmentInput::Conversation {
        system: None,
        messages: Vec::new(),
    };

    assert_eq!(
        serde_json::to_value(input).unwrap(),
        json!({"messages": []})
    );
    for encoded in [
        json!({"system": " \t\n", "messages": []}),
        json!({"messages": []}),
    ] {
        assert_eq!(
            serde_json::from_value::<JudgmentInput>(encoded).unwrap(),
            absent
        );
    }
}

#[test]
fn validation_rejects_a_lone_tool_result() {
    assert_missing_tool_call(validate_messages(vec![tool_result()]));
}

#[test]
fn validation_accepts_a_tool_result_after_its_call() {
    validate_messages(vec![
        tool_call("call-1"),
        ConversationMessage::user("Check the invoice."),
        tool_result(),
    ])
    .unwrap();
}

#[test]
fn validation_rejects_a_tool_result_whose_call_comes_later() {
    assert_missing_tool_call(validate_messages(vec![
        tool_call("call-2"),
        tool_result(),
        tool_call("call-1"),
    ]));
}

#[test]
fn validation_accepts_a_tool_call_without_a_result() {
    validate_messages(vec![tool_call("call-1")]).unwrap();
}

fn validate_messages(messages: Vec<ConversationMessage>) -> JudgmentResult<()> {
    JudgmentRequest {
        input: JudgmentInput::Conversation {
            system: None,
            messages,
        },
        questions: BTreeMap::from([(
            "urgent".to_owned(),
            JudgmentQuestion::condition("Is the invoice overdue?", None),
        )]),
    }
    .validate()
}

fn tool_call(id: &str) -> ConversationMessage {
    ConversationMessage::assistant(
        "",
        vec![ToolCall {
            id: id.to_owned(),
            name: "lookup".to_owned(),
            input: json!({"invoice": 1}),
            operation_id: None,
        }],
    )
}

fn tool_result() -> ConversationMessage {
    ConversationMessage::tool("fixture output", "lookup", "call-1")
}

fn assert_missing_tool_call(result: JudgmentResult<()>) {
    assert!(
        matches!(&result, Err(JudgmentError::Failure { core })
            if core.kind == ErrorKind::InvalidRequest
                && core.provider == "local"
                && core.model_id == "judgment-validation"
                && core.message == "tool result has no earlier tool call with id `call-1`"),
        "{result:?}"
    );
}
