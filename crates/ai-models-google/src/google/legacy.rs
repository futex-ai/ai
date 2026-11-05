//! Provider-owned conversion of stored Gemini assistant content.

use ai_interface::{
    ConversationPart, ConversationRole, LegacyConversationMessage, LegacyPartAttachment,
    LegacyPartTarget, LegacyProviderConverter, LegacyProviderItem, ProviderId, ProviderState,
    ToolCall,
};
use serde_json::{Map, Value};

use crate::PROVIDER_ID;

use super::state::{GoogleAssistantPart, GooglePartState};

#[derive(Debug, Default)]
/// Converts version one Gemini replay items into ordered conversation parts.
pub struct GoogleLegacyProviderConverter;

impl LegacyProviderConverter for GoogleLegacyProviderConverter {
    fn provider(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn convert_item(
        &self,
        item: &LegacyProviderItem,
        message: &LegacyConversationMessage,
        _item_index: usize,
    ) -> Option<Vec<LegacyPartAttachment>> {
        if item.kind != "google_assistant_message" {
            return None;
        }
        let state = ProviderState {
            provider: PROVIDER_ID,
            model: String::new(),
            version: 1,
            payload: item.fields.clone(),
        };
        let decoded =
            match serde_json::from_value::<GooglePartState>(Value::Object(item.fields.clone())) {
                Ok(decoded) => decoded,
                Err(_) => {
                    return Some(vec![LegacyPartAttachment {
                        target: LegacyPartTarget::Reasoning {
                            text: String::new(),
                        },
                        provider_state: state,
                    }]);
                }
            };
        let mut calls = message.tool_calls.iter();
        let mut attachments = Vec::new();
        for part in decoded.parts {
            let target = if let Some(original_call) = &part.function_call {
                let call_id = calls
                    .next()
                    .map(|call| call.id.clone())
                    .or_else(|| original_call.id.clone())
                    .unwrap_or_default();
                LegacyPartTarget::ToolCall { call_id }
            } else if part.thought == Some(true) || part.text.is_none() {
                LegacyPartTarget::Reasoning {
                    text: part.text.clone().unwrap_or_default(),
                }
            } else {
                LegacyPartTarget::Text
            };
            let provider_state = GooglePartState::new(vec![part]).encode("").ok()?;
            attachments.push(LegacyPartAttachment {
                target,
                provider_state,
            });
        }
        Some(attachments)
    }

    fn convert_message(
        &self,
        message: &LegacyConversationMessage,
    ) -> Option<Vec<ConversationPart>> {
        if message.role != ConversationRole::Assistant {
            return None;
        }
        let item = message
            .provider_context
            .iter()
            .find(|item| item.kind == "google_assistant_message")?;
        let decoded: GooglePartState =
            serde_json::from_value(Value::Object(item.fields.clone())).ok()?;
        let mut calls = message.tool_calls.iter();
        let mut parts = Vec::new();
        for original in decoded.parts {
            let part = portable_part(&original, &mut calls);
            parts.push(
                part.with_provider_state(GooglePartState::new(vec![original]).encode("").ok()?),
            );
        }
        preserve_portable_text(&message.content, &mut parts);
        parts.extend(
            calls.cloned().map(|call| {
                ConversationPart::from_tool_call(call).with_provider_state(empty_state())
            }),
        );
        if parts.is_empty() {
            parts.push(ConversationPart::text("").with_provider_state(empty_state()));
        }
        Some(parts)
    }
}

fn portable_part(
    original: &GoogleAssistantPart,
    calls: &mut std::slice::Iter<'_, ToolCall>,
) -> ConversationPart {
    if original.function_call.is_some() {
        return match calls.next() {
            Some(call) => ConversationPart::from_tool_call(call.clone()),
            None => ConversationPart::reasoning(""),
        };
    }
    let text = original.text.clone().unwrap_or_default();
    if original.thought == Some(true) || original.text.is_none() {
        ConversationPart::reasoning(text)
    } else {
        ConversationPart::text(text)
    }
}

fn preserve_portable_text(content: &str, parts: &mut Vec<ConversationPart>) {
    let mut remaining = Some(content.to_owned());
    for part in parts.iter_mut() {
        if let ConversationPart::Text { text, .. } = part {
            *text = remaining.take().unwrap_or_default();
        }
    }
    if let Some(text) = remaining
        && !text.is_empty()
    {
        parts.push(ConversationPart::text(text).with_provider_state(empty_state()));
    }
}

fn empty_state() -> ProviderState {
    ProviderState {
        provider: PROVIDER_ID,
        model: String::new(),
        version: 1,
        payload: Map::from_iter([
            ("scope".to_owned(), Value::String("part".to_owned())),
            ("parts".to_owned(), Value::Array(Vec::new())),
        ]),
    }
}
