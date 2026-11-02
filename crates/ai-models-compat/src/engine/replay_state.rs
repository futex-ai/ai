//! Private typed replay payloads and versioned part-state encoding.

use ai_interface::{ErrorKind, ModelError, ModelResult, ProviderId, ProviderState};
use ai_models_core::ErrorContext;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{deepseek, kimi, minimax, minimax::state::ReasoningDetail, qwen, xai};

use super::dialect::{Dialect, ReplayRule};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RawToolCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PartState {
    Text {
        content: Option<String>,
    },
    Reasoning {
        reasoning_content: Option<String>,
        #[serde(default)]
        reasoning_details: Vec<ReasoningDetail>,
    },
    ToolCall {
        call: RawToolCall,
        legacy: bool,
    },
}

pub(crate) enum DecodedState {
    Part(PartState),
    Assistant {
        content: Option<String>,
        reasoning_content: Option<String>,
        tool_calls: Vec<RawToolCall>,
    },
    Reasoning {
        reasoning_content: Option<String>,
        reasoning_details: Vec<ReasoningDetail>,
    },
    LegacyCall {
        tool_call_id: String,
        name: String,
        arguments: String,
    },
}

pub(crate) fn decode(
    dialect: &Dialect,
    model_id: &str,
    state: &ProviderState,
) -> ModelResult<Option<DecodedState>> {
    let provider = ProviderId::new(dialect.provider);
    if state.provider != provider {
        return Ok(None);
    }
    if state.version == 2 {
        return Ok(state
            .decode(&provider, model_id, 2)?
            .map(DecodedState::Part));
    }
    match dialect.replay {
        ReplayRule::DeepSeek => deepseek::state::decode(state, model_id),
        ReplayRule::Kimi => kimi::state::decode(state, model_id),
        ReplayRule::Qwen => qwen::state::decode(state, model_id),
        ReplayRule::MiniMax => minimax::state::decode(state, model_id),
        ReplayRule::Xai => xai::state::decode(state, model_id),
    }
}

pub(crate) fn encode(
    context: &ErrorContext,
    dialect: &Dialect,
    model_id: &str,
    payload: PartState,
) -> ModelResult<ProviderState> {
    let value = match serde_json::to_value(payload) {
        Ok(value) => value,
        Err(source) => {
            return Err(ModelError::with_source(
                context.core(ErrorKind::Internal, "internal error"),
                source,
            ));
        }
    };
    let Value::Object(payload) = value else {
        return Err(context.model_error(ErrorKind::Internal, "invalid provider state payload"));
    };
    Ok(ProviderState {
        provider: ProviderId::new(dialect.provider),
        model: model_id.to_owned(),
        version: 2,
        payload,
    })
}
