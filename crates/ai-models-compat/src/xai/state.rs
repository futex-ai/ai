//! xAI private legacy function-call state and old-message conversion.

use ai_interface::{
    LegacyConversationMessage, LegacyPartAttachment, LegacyPartTarget, LegacyProviderConverter,
    LegacyProviderItem, ModelResult, ProviderId, ProviderState,
};
use serde::Deserialize;

use crate::engine::replay_state::DecodedState;

use super::PROVIDER_ID;

#[derive(Deserialize)]
struct LegacyCallState {
    tool_call_id: String,
    name: String,
    arguments: String,
}

pub(crate) fn decode(state: &ProviderState, model_id: &str) -> ModelResult<Option<DecodedState>> {
    let Some(payload) = state.decode::<LegacyCallState>(&PROVIDER_ID, model_id, 1)? else {
        return Ok(None);
    };
    Ok(Some(DecodedState::LegacyCall {
        tool_call_id: payload.tool_call_id,
        name: payload.name,
        arguments: payload.arguments,
    }))
}

/// Converts stored version-one xAI legacy function calls.
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
        (item.kind == "xai_legacy_function_call").then(|| {
            vec![LegacyPartAttachment {
                target: LegacyPartTarget::ToolCall {
                    call_id: item
                        .fields
                        .get("tool_call_id")
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
