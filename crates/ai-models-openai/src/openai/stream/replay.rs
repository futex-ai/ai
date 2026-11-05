//! Provider state produced by native Responses message and reasoning items.

use ai_interface::{ModelResult, ProviderState};

use crate::openai::{
    replay_state::{MessageState, ReasoningState, ReplayState},
    response::types::ResponsesOutputItem,
};

pub(super) fn item_state(
    item: &ResponsesOutputItem,
    model_id: &str,
    preserve_message: bool,
) -> ModelResult<Option<ProviderState>> {
    let state = match item {
        ResponsesOutputItem::Message { phase, .. } if preserve_message || phase.is_some() => {
            ReplayState::Message(MessageState {
                phase: phase.clone(),
            })
        }
        ResponsesOutputItem::Reasoning {
            id,
            summary,
            encrypted_content,
        } => ReplayState::Reasoning(ReasoningState {
            id: id.clone(),
            summary: summary.clone(),
            encrypted_content: encrypted_content.clone(),
        }),
        _ => return Ok(None),
    };
    Ok(Some(state.encode(model_id)?))
}
