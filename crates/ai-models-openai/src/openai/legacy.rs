//! One-way conversion of version one Responses replay items.

use ai_interface::{
    LegacyConversationMessage, LegacyPartAttachment, LegacyPartOrder, LegacyPartTarget,
    LegacyProviderConverter, LegacyProviderItem, ProviderId, ProviderState,
};

use crate::PROVIDER_ID;

use super::replay_state::{ReplayState, SkipReplay, SkipState};

#[derive(Clone, Copy, Debug, Default)]
/// Converts stored version one OpenAI items into ordered conversation parts.
pub struct OpenAiLegacyConverter;

impl LegacyProviderConverter for OpenAiLegacyConverter {
    fn provider(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn order(&self) -> LegacyPartOrder {
        LegacyPartOrder::Context
    }

    fn convert_item(
        &self,
        item: &LegacyProviderItem,
        message: &LegacyConversationMessage,
        item_index: usize,
    ) -> Option<Vec<LegacyPartAttachment>> {
        let target = match item.kind.as_str() {
            "openai_message" => {
                if message
                    .provider_context
                    .iter()
                    .take(item_index)
                    .any(|item| item.kind == "openai_message")
                {
                    return Some(Vec::new());
                }
                LegacyPartTarget::Text
            }
            "openai_reasoning" => LegacyPartTarget::Reasoning {
                text: String::new(),
            },
            "openai_function_call" => {
                match item.fields.get("call_id").and_then(|value| value.as_str()) {
                    Some(call_id) if message.tool_calls.iter().any(|call| call.id == call_id) => {
                        LegacyPartTarget::ToolCall {
                            call_id: call_id.to_owned(),
                        }
                    }
                    Some(_) => LegacyPartTarget::Reasoning {
                        text: String::new(),
                    },
                    None => {
                        return Some(vec![LegacyPartAttachment {
                            target: LegacyPartTarget::ToolCall {
                                call_id: String::new(),
                            },
                            provider_state: ProviderState {
                                provider: PROVIDER_ID,
                                model: String::new(),
                                version: 1,
                                payload: item.fields.clone(),
                            },
                        }]);
                    }
                }
            }
            _ => return None,
        };
        let mut attachments = vec![LegacyPartAttachment {
            target,
            provider_state: ProviderState {
                provider: PROVIDER_ID,
                model: String::new(),
                version: 1,
                payload: item.fields.clone(),
            },
        }];
        if item.kind == "openai_function_call"
            && !message
                .provider_context
                .iter()
                .take(item_index)
                .any(|item| item.kind == "openai_function_call")
        {
            let state = ReplayState::Skip(SkipState {
                replay: SkipReplay::Skip,
            })
            .encode("")
            .ok()?;
            for call in &message.tool_calls {
                if !message.provider_context.iter().any(|item| {
                    item.kind == "openai_function_call"
                        && item.fields.get("call_id").and_then(|value| value.as_str())
                            == Some(call.id.as_str())
                }) {
                    attachments.push(LegacyPartAttachment {
                        target: LegacyPartTarget::UnorderedToolCall {
                            call_id: call.id.clone(),
                        },
                        provider_state: state.clone(),
                    });
                }
            }
        }
        Some(attachments)
    }
}
