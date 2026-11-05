//! Per-part replay state and reasoning retained by the public fold.

use serde_json::json;

use crate::{ConversationPart, FinishReason, ModelEvent, ProviderId, ProviderState};

use super::fixtures::{arguments, finish, fold, text, tool_start, usage};

fn state(token: &str) -> ProviderState {
    ProviderState {
        provider: ProviderId::new("sample"),
        model: "model-one".to_owned(),
        version: 1,
        payload: json!({"token":token}).as_object().unwrap().clone(),
    }
}

#[test]
fn fold_keeps_latest_state_per_part_and_all_reasoning_text() {
    let response = fold(vec![
        ModelEvent::ReasoningDelta {
            part_index: 0,
            delta: String::new(),
            provider_state: Some(state("first")),
        },
        ModelEvent::TextDelta {
            part_index: 1,
            delta: "first".to_owned(),
            provider_state: Some(state("text")),
        },
        ModelEvent::ReasoningDelta {
            part_index: 0,
            delta: "thought".to_owned(),
            provider_state: Some(state("latest")),
        },
        text(1, " last"),
        tool_start(2, "one"),
        arguments(2, "{}"),
        ModelEvent::ToolCallEnd {
            part_index: 2,
            provider_state: Some(state("call")),
        },
        usage(),
        finish(FinishReason::ToolCalls),
    ])
    .unwrap();
    assert_eq!(
        response.parts,
        vec![
            ConversationPart::reasoning("thought").with_provider_state(state("latest")),
            ConversationPart::text("first last").with_provider_state(state("text")),
            ConversationPart::ToolCall {
                call_id: "one".to_owned(),
                tool_name: "lookup".to_owned(),
                arguments: json!({}),
                operation_id: Some("operation-one".to_owned()),
                provider_state: Some(state("call"))
            }
        ]
    );
    assert_eq!(response.text(), "first last");
    assert_eq!(response.to_assistant_message().parts, response.parts);
}
