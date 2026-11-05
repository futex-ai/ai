//! Anonymous fragments retain their text and the native replay-item order.

use ai_interface::{
    ConversationPart, Effort, ModelError, ModelResponseFold, StructuredOutputSchema,
};
use serde_json::json;

use super::{state::OpenAiStreamState, types::OpenAiStreamEvent};

#[test]
fn anonymous_text_waits_for_terminal_replay_item_order() {
    let mut state = OpenAiStreamState::new("gpt-5.5");
    let first = state
        .push(
            serde_json::from_value::<OpenAiStreamEvent>(
                json!({"type":"response.output_text.delta","delta":"Checking now"}),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(first.len(), 0);
    let events = state.complete("openai/gpt-5.5","gpt-5.5",Some(Effort::Disabled),serde_json::from_value(json!({"status":"completed","output":[
        {"type":"reasoning","id":"rs-one","summary":[],"encrypted_content":"signed"},
        {"type":"message","phase":"commentary","content":[{"type":"output_text","text":"Checking now"}]}
    ]})).unwrap(),None).unwrap();
    let mut fold = ModelResponseFold::new();
    for event in events {
        fold.push(event).unwrap();
    }
    let response = fold.finish().unwrap();
    assert_eq!(response.text(), "Checking now");
    assert!(
        matches!(&response.parts[..],[ConversationPart::Reasoning { text,provider_state:Some(reasoning) },ConversationPart::Text { provider_state:Some(message),.. }] if text.is_empty() && reasoning.payload["encrypted_content"] == "signed" && message.payload["phase"] == "commentary")
    );
}

#[test]
fn anonymous_fragments_survive_terminal_output_validation_errors() {
    let mut state = OpenAiStreamState::new("gpt-5.5");
    state
        .push(
            serde_json::from_value(json!({"type":"response.output_text.delta","delta":"{"}))
                .unwrap(),
        )
        .unwrap();
    let error = state.complete("openai/gpt-5.5","gpt-5.5",Some(Effort::Disabled),serde_json::from_value(json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{"}]}]})).unwrap(),Some(&StructuredOutputSchema { name:"answer".to_owned(),schema:json!({"type":"object"}) }));
    assert!(matches!(error, Err(ModelError::Provider { .. })));
    let pending = state
        .flush_anonymous()
        .unwrap()
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        pending,
        vec![json!({"TextDelta":{"part_index":0,"delta":"{"}})]
    );
}
