//! Judgment input and stored conversation serde tests.

use serde_json::json;

use crate::{ConversationMessage, JudgmentInput};

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
