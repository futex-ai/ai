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
        Some(parts)
    }
}

fn portable_part(
    original: &GoogleAssistantPart,
    calls: &mut std::slice::Iter<'_, ToolCall>,
) -> ConversationPart {
    if let Some(call) = &original.function_call {
        let call = calls.next().cloned().unwrap_or_else(|| ToolCall {
            id: call.id.clone().unwrap_or_default(),
            name: call.name.clone(),
            input: call
                .args
                .clone()
                .unwrap_or_else(|| Value::Object(Map::new())),
            operation_id: None,
        });
        return ConversationPart::from_tool_call(call);
    }
    let text = original.text.clone().unwrap_or_default();
    if original.thought == Some(true) || original.text.is_none() {
        ConversationPart::reasoning(text)
    } else {
        ConversationPart::text(text)
    }
}
