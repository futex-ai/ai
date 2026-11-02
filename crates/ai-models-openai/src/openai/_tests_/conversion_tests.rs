//! Stored Responses conversion preserves context order and raw replay fields.

use std::slice;

use ai_interface::{ConversationMessage, ConversationPart, read_stored_message};
use serde_json::json;

use super::{OpenAiLegacyConverter, parts_tests::mapped};

#[test]
fn legacy_converter_keeps_first_phase_item_order_and_raw_arguments() {
    let message = read_stored_message(json!({"role":"assistant","content":"visible","name":"helper","tool_calls":[
        {"id":"a","name":"first","input":{},"operation_id":"operation-a"},{"id":"b","name":"second","input":{"id":7}}
    ],"provider_context":[
        {"type":"openai_reasoning","id":"rs-one","summary":[{"type":"summary_text","text":"thought"}],"encrypted_content":"signed"},
        {"type":"openai_function_call","id":"fc-b","call_id":"b","name":"second","arguments":"{\n  \"id\": 7\n}"},
        {"type":"openai_message","phase":"commentary"},{"type":"openai_message","phase":"final_answer"}
    ]}), &[&OpenAiLegacyConverter]).unwrap();
    assert_eq!(
        mapped(slice::from_ref(&message)),
        json!([
            {"type":"reasoning","id":"rs-one","summary":[{"type":"summary_text","text":"thought"}],"encrypted_content":"signed"},
            {"type":"function_call","id":"fc-b","call_id":"b","name":"second","arguments":"{\n  \"id\": 7\n}"},
            {"role":"assistant","phase":"commentary","content":"visible"}
        ])
    );
    assert_eq!(message.name.as_deref(), Some("helper"));
    assert_eq!(
        message.tool_calls()[1].operation_id.as_deref(),
        Some("operation-a")
    );
    assert!(matches!(&message.parts[0],ConversationPart::Reasoning { text,.. } if text.is_empty()));
}

#[test]
fn legacy_converter_keeps_native_calls_without_matching_portable_parts() {
    let message = read_stored_message(
        json!({"role":"assistant","content":"","tool_calls":[],"provider_context":[
            {"type":"openai_message","phase":"commentary"},
            {"type":"openai_function_call","call_id":"missing","name":"search","arguments":"{}"}
        ]}),
        &[&OpenAiLegacyConverter],
    )
    .unwrap();
    assert_eq!(message.tool_calls(), Vec::new());
    assert!(
        matches!(message.parts.as_slice(), [ConversationPart::Reasoning { text, .. }] if text.is_empty())
    );
    assert_eq!(
        mapped(&[message]),
        json!([{"type":"function_call","call_id":"missing","name":"search","arguments":"{}"}])
    );
}

#[test]
fn legacy_converter_keeps_unmatched_portable_calls_after_text_without_replay() {
    for phase in [None, Some("commentary")] {
        let mut context = vec![
            json!({"type":"openai_function_call","id":"fc-b","call_id":"b","name":"second","arguments":"{}"}),
        ];
        if let Some(phase) = phase {
            context.push(json!({"type":"openai_message","phase":phase}));
        }
        let message = read_stored_message(
            json!({"role":"assistant","content":"visible","tool_calls":[
                {"id":"a","name":"first","input":{},"operation_id":"operation-a"},
                {"id":"b","name":"second","input":{}}
            ],"provider_context":context}),
            &[&OpenAiLegacyConverter],
        )
        .unwrap();
        let message: ConversationMessage =
            serde_json::from_value(serde_json::to_value(message).unwrap()).unwrap();
        assert!(matches!(message.parts.as_slice(), [
            ConversationPart::ToolCall { call_id: first, .. },
            ConversationPart::Text { text, .. },
            ConversationPart::ToolCall { call_id: last, operation_id: Some(operation), .. }
        ] if first == "b" && text == "visible" && last == "a" && operation == "operation-a"));
        let mut expected_message = json!({"role":"assistant","content":"visible"});
        if let Some(phase) = phase {
            expected_message["phase"] = json!(phase);
        }
        assert_eq!(
            mapped(&[message]),
            json!([
                {"type":"function_call","id":"fc-b","call_id":"b","name":"second","arguments":"{}"},
                expected_message
            ]),
            "phase: {phase:?}"
        );
    }
}
