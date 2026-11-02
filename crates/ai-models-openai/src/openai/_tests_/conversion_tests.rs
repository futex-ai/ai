//! Stored Responses conversion preserves context order and raw replay fields.

use std::slice;

use ai_interface::{ConversationPart, read_stored_message};
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
            {"role":"assistant","phase":"commentary","content":"visible"},
            {"type":"function_call","call_id":"a","name":"first","arguments":"{}"}
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
fn legacy_converter_drops_items_without_matching_portable_parts() {
    let message = read_stored_message(
        json!({"role":"assistant","content":"","tool_calls":[],"provider_context":[
            {"type":"openai_message","phase":"commentary"},
            {"type":"openai_function_call","call_id":"missing","name":"search","arguments":"{}"}
        ]}),
        &[&OpenAiLegacyConverter],
    )
    .unwrap();
    assert_eq!(message.parts, Vec::new());
}
