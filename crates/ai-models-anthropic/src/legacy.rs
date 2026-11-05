//! Conversion of Anthropic version one assistant blocks to ordered parts.

use ai_interface::{
    ConversationPart, ConversationRole, LegacyConversationMessage, LegacyPartAttachment,
    LegacyPartTarget, LegacyProviderConverter, LegacyProviderItem, ProviderId, ProviderState,
};
use serde_json::{Map, Value};

use crate::{
    PROVIDER_ID,
    anthropic::replay::{AnthropicAssistantBlock, AnthropicReplay, ReplayMode, ReplayScope},
};

#[derive(Clone, Copy, Debug, Default)]
/// Converts stored Anthropic replay blocks without changing their request order.
pub struct AnthropicLegacyConverter;

impl LegacyProviderConverter for AnthropicLegacyConverter {
    fn provider(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn convert_item(
        &self,
        item: &LegacyProviderItem,
        _message: &LegacyConversationMessage,
        _item_index: usize,
    ) -> Option<Vec<LegacyPartAttachment>> {
        let replay = legacy_replay(item)?;
        replay
            .blocks
            .into_iter()
            .map(|block| {
                let target = match &block {
                    AnthropicAssistantBlock::Thinking { thinking, .. } => {
                        LegacyPartTarget::Reasoning {
                            text: thinking.clone(),
                        }
                    }
                    AnthropicAssistantBlock::RedactedThinking { .. } => {
                        LegacyPartTarget::Reasoning {
                            text: String::new(),
                        }
                    }
                    AnthropicAssistantBlock::Text { .. } => LegacyPartTarget::Text,
                    AnthropicAssistantBlock::ToolUse { id, .. } => LegacyPartTarget::ToolCall {
                        call_id: id.clone(),
                    },
                };
                Some(LegacyPartAttachment {
                    target,
                    provider_state: block.provider_state("", ReplayMode::Original).ok()?,
                })
            })
            .collect()
    }

    fn convert_message(
        &self,
        message: &LegacyConversationMessage,
    ) -> Option<Vec<ConversationPart>> {
        if message.role != ConversationRole::Assistant {
            return None;
        }
        let replay = message.provider_context.iter().find_map(legacy_replay)?;
        let mut emitted_text = false;
        let mut parts = replay
            .blocks
            .into_iter()
            .map(|block| {
                let state = block.provider_state("", ReplayMode::Original).ok()?;
                let part = match block {
                    AnthropicAssistantBlock::Thinking { thinking, .. } => {
                        ConversationPart::reasoning(thinking)
                    }
                    AnthropicAssistantBlock::RedactedThinking { .. } => {
                        ConversationPart::reasoning("")
                    }
                    AnthropicAssistantBlock::Text { text } => {
                        let text = if text.trim().is_empty() {
                            String::new()
                        } else if emitted_text {
                            format!("\n{text}")
                        } else {
                            emitted_text = true;
                            text
                        };
                        ConversationPart::text(text)
                    }
                    AnthropicAssistantBlock::ToolUse { id, .. } => {
                        let call = message
                            .tool_calls
                            .iter()
                            .find(|call| call.id == id)
                            .cloned();
                        match call {
                            Some(call) => ConversationPart::from_tool_call(call),
                            None => ConversationPart::reasoning(""),
                        }
                    }
                };
                Some(part.with_provider_state(state))
            })
            .collect::<Option<Vec<_>>>()?;
        preserve_portable_parts(message, &mut parts);
        Some(parts)
    }
}

fn preserve_portable_parts(message: &LegacyConversationMessage, parts: &mut Vec<ConversationPart>) {
    let original_text: String = parts
        .iter()
        .filter_map(|part| match part {
            ConversationPart::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    if original_text != message.content {
        let mut text_part_exists = false;
        for part in parts.iter_mut() {
            if let ConversationPart::Text { text, .. } = part {
                *text = if text_part_exists {
                    String::new()
                } else {
                    message.content.clone()
                };
                text_part_exists = true;
            }
        }
        if !text_part_exists {
            parts.push(
                ConversationPart::text(message.content.clone()).with_provider_state(empty_state()),
            );
        }
    }
    for call in &message.tool_calls {
        if !parts
            .iter()
            .filter_map(ConversationPart::as_tool_call)
            .any(|part| part.id == call.id)
        {
            parts.push(
                ConversationPart::from_tool_call(call.clone()).with_provider_state(empty_state()),
            );
        }
    }
    if parts.is_empty() {
        parts.push(ConversationPart::text("").with_provider_state(empty_state()));
    }
}

fn empty_state() -> ProviderState {
    ProviderState {
        provider: PROVIDER_ID,
        model: String::new(),
        version: 1,
        payload: Map::from_iter([
            ("blocks".to_owned(), Value::Array(Vec::new())),
            ("scope".to_owned(), Value::from(ReplayScope::Part)),
        ]),
    }
}

fn legacy_replay(item: &LegacyProviderItem) -> Option<AnthropicReplay> {
    if item.kind != "anthropic_assistant_message" {
        return None;
    }
    serde_json::from_value(Value::Object(item.fields.clone())).ok()
}
