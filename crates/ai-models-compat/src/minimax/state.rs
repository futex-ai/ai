//! MiniMax private reasoning replay state and old-message conversion.

use ai_interface::{
    LegacyConversationMessage, LegacyPartAttachment, LegacyPartTarget, LegacyProviderConverter,
    LegacyProviderItem, ModelResult, ProviderId, ProviderState,
};
use serde::{Deserialize, Serialize};

use crate::engine::replay_state::DecodedState;

use super::PROVIDER_ID;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ReasoningDetail {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
}

#[derive(Deserialize)]
struct ReasoningState {
    reasoning_content: Option<String>,
    #[serde(default)]
    reasoning_details: Vec<ReasoningDetail>,
}

pub(crate) fn decode(state: &ProviderState, model_id: &str) -> ModelResult<Option<DecodedState>> {
    let Some(payload) = state.decode::<ReasoningState>(&PROVIDER_ID, model_id, 1)? else {
        return Ok(None);
    };
    Ok(Some(DecodedState::Reasoning {
        reasoning_content: payload.reasoning_content,
        reasoning_details: payload.reasoning_details,
    }))
}

/// Converts stored version-one MiniMax reasoning replay items.
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
        (item.kind == "minimax_assistant").then(|| {
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
