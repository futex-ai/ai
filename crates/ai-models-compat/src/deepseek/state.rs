//! DeepSeek version-one assistant replay state and conversion.

use ai_interface::{
    LegacyConversationMessage, LegacyPartAttachment, LegacyPartTarget, LegacyProviderConverter,
    LegacyProviderItem, ModelResult, ProviderId, ProviderState,
};
use serde::Deserialize;

use crate::engine::replay_state::{DecodedState, RawToolCall};

use super::PROVIDER_ID;

#[derive(Deserialize)]
struct AssistantState {
    content: String,
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<RawToolCall>,
}

pub(crate) fn decode(state: &ProviderState, model_id: &str) -> ModelResult<Option<DecodedState>> {
    let Some(payload) = state.decode::<AssistantState>(&PROVIDER_ID, model_id, 1)? else {
        return Ok(None);
    };
    Ok(Some(DecodedState::Assistant {
        content: Some(payload.content),
        reasoning_content: payload.reasoning_content,
        tool_calls: payload.tool_calls,
    }))
}

/// Converts stored version-one DeepSeek assistant replay items.
pub struct LegacyConverter;

impl LegacyProviderConverter for LegacyConverter {
    fn provider(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn convert_item(
        &self,
        item: &LegacyProviderItem,
        _message: &LegacyConversationMessage,
        _item_index: usize,
    ) -> Option<Vec<LegacyPartAttachment>> {
        (item.kind == "deep_seek_assistant_message").then(|| {
            vec![LegacyPartAttachment {
                target: LegacyPartTarget::Reasoning {
                    text: item
                        .fields
                        .get("reasoning_content")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                },
                provider_state: ProviderState {
                    provider: PROVIDER_ID,
                    model: String::new(),
                    version: 1,
                    payload: item.fields.clone(),
                },
            }]
        })
    }
}
