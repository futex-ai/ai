//! Provider conversion composition, ordering, and preservation of unknown state.

use std::sync::Arc;

use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    ConversationPart, LegacyMessageConverter, LegacyPartAttachment, LegacyPartOrder,
    LegacyPartTarget, LegacyProviderConverterMock, ProviderId, ProviderState, read_stored_message,
};

fn converter(provider: &'static str, order: LegacyPartOrder) -> Unimock {
    let items = LegacyProviderConverterMock::convert_item
        .each_call(matching!(_, _, _))
        .answers_arc(Arc::new(move |_, item, _, _| {
            if !item.kind.starts_with(provider) {
                return None;
            }
            let target = match item.fields.get("call_id").and_then(|value| value.as_str()) {
                Some(call_id) => LegacyPartTarget::ToolCall {
                    call_id: call_id.to_owned(),
                },
                None if item.kind.ends_with("_message") => LegacyPartTarget::Text,
                None => LegacyPartTarget::Reasoning {
                    text: "thought".to_owned(),
                },
            };
            Some(vec![LegacyPartAttachment {
                target,
                provider_state: ProviderState {
                    provider: ProviderId::new(provider),
                    model: String::new(),
                    version: 1,
                    payload: item.fields.clone(),
                },
            }])
        }));
    if provider == "sample" {
        Unimock::new((
            items,
            LegacyProviderConverterMock::order
                .each_call(matching!())
                .returns(order),
            LegacyProviderConverterMock::convert_message
                .each_call(matching!(_))
                .returns(None),
        ))
    } else {
        Unimock::new(items)
    }
}

#[test]
fn first_items_provider_sets_mixed_message_order() {
    let sample = converter("sample", LegacyPartOrder::Context);
    let other = converter("other", LegacyPartOrder::ReasoningFirst);
    let composition = LegacyMessageConverter::new(vec![Arc::new(other), Arc::new(sample)]);
    let message = composition.read(json!({"role":"assistant","content":"visible","tool_calls":[
        {"id":"a","name":"first","input":{}},{"id":"b","name":"second","input":{}}
    ],"provider_context":[{"type":"sample_function_call","call_id":"b"},{"type":"other_reasoning","token":"signed"}] })).unwrap();
    assert_eq!(
        message.parts,
        vec![
            ConversationPart::ToolCall {
                call_id: "b".to_owned(),
                tool_name: "second".to_owned(),
                arguments: json!({}),
                operation_id: None,
                provider_state: Some(ProviderState {
                    provider: ProviderId::new("sample"),
                    model: String::new(),
                    version: 1,
                    payload: json!({"call_id":"b"}).as_object().unwrap().clone()
                })
            },
            ConversationPart::reasoning("thought").with_provider_state(ProviderState {
                provider: ProviderId::new("other"),
                model: String::new(),
                version: 1,
                payload: json!({"token":"signed"}).as_object().unwrap().clone()
            }),
            ConversationPart::text("visible"),
            ConversationPart::ToolCall {
                call_id: "a".to_owned(),
                tool_name: "first".to_owned(),
                arguments: json!({}),
                operation_id: None,
                provider_state: None
            }
        ]
    );
}

#[test]
fn unknown_items_keep_replay_data_when_the_target_is_missing() {
    let message = read_stored_message(json!({"role":"assistant","content":"visible","provider_context":[{"type":"not_installed_function_call","call_id":"missing","arguments":"{ raw }"}]}), &[]).unwrap();
    assert_eq!(
        message.parts,
        vec![
            ConversationPart::reasoning("").with_provider_state(ProviderState {
                provider: ProviderId::new("notinstalled"),
                model: String::new(),
                version: 1,
                payload: json!({"call_id":"missing","arguments":"{ raw }"})
                    .as_object()
                    .unwrap()
                    .clone()
            }),
            ConversationPart::text("visible")
        ]
    );
}

#[test]
fn registered_unmatched_items_drop_and_occupied_parts_keep_both_states() {
    let sample = converter("sample", LegacyPartOrder::ReasoningFirst);
    let other = converter("other", LegacyPartOrder::ReasoningFirst);
    let message = read_stored_message(json!({"role":"assistant","content":"visible","provider_context":[
        {"type":"sample_message","token":"primary"},{"type":"other_message","token":"foreign"},{"type":"sample_function_call","call_id":"missing"}
    ]}), &[&sample,&other]).unwrap();
    assert_eq!(
        message.parts,
        vec![
            ConversationPart::reasoning("").with_provider_state(ProviderState {
                provider: ProviderId::new("other"),
                model: String::new(),
                version: 1,
                payload: json!({"token":"foreign"}).as_object().unwrap().clone()
            }),
            ConversationPart::text("visible").with_provider_state(ProviderState {
                provider: ProviderId::new("sample"),
                model: String::new(),
                version: 1,
                payload: json!({"token":"primary"}).as_object().unwrap().clone()
            })
        ]
    );
}
