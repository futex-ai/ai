//! Private versioned Anthropic block state for exact signed continuations.

use ai_interface::{ModelError, ModelResult, ProviderState};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::PROVIDER_ID;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum AnthropicAssistantBlock {
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct AnthropicReplay {
    pub(crate) blocks: Vec<AnthropicAssistantBlock>,
    #[serde(default, skip_serializing_if = "ReplayMode::is_original")]
    pub(crate) mode: ReplayMode,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReplayMode {
    /// Replays original blocks, including empty signed message boundaries.
    #[default]
    Original,
    /// Keeps raw text for signed replay and portable text for unsigned replay.
    NormalizedText,
}

impl ReplayMode {
    fn is_original(&self) -> bool {
        matches!(self, Self::Original)
    }
}

impl AnthropicReplay {
    pub(super) fn decode(state: &ProviderState, model_id: &str) -> ModelResult<Option<Self>> {
        if state.provider != PROVIDER_ID {
            return Ok(None);
        }
        if state.version != 1 {
            return Err(invalid_state(
                model_id,
                ReplayError::UnsupportedVersion {
                    version: state.version,
                },
            ));
        }
        match serde_json::from_value(Value::Object(state.payload.clone())) {
            Ok(replay) => Ok(Some(replay)),
            Err(source) => Err(invalid_state(
                model_id,
                ReplayError::InvalidPayload { source },
            )),
        }
    }
}

impl AnthropicAssistantBlock {
    pub(super) fn is_reasoning(&self) -> bool {
        matches!(self, Self::Thinking { .. } | Self::RedactedThinking { .. })
    }

    pub(crate) fn provider_state(
        &self,
        model_id: &str,
        mode: ReplayMode,
    ) -> Result<ProviderState, serde_json::Error> {
        let mut payload = Map::from_iter([(
            "blocks".to_owned(),
            Value::Array(vec![serde_json::to_value(self)?]),
        )]);
        if mode == ReplayMode::NormalizedText {
            payload.insert("mode".to_owned(), serde_json::to_value(mode)?);
        }
        Ok(ProviderState {
            provider: PROVIDER_ID,
            model: model_id.to_owned(),
            version: 1,
            payload,
        })
    }
}

fn invalid_state(model_id: &str, source: ReplayError) -> ModelError {
    ModelError::provider(PROVIDER_ID.as_str(), model_id, source.to_string())
}

#[derive(Debug, Error)]
enum ReplayError {
    #[error("[ai_models_anthropic/replay] unsupported provider state version {version}")]
    UnsupportedVersion { version: u32 },
    #[error("[ai_models_anthropic/replay] invalid provider state payload")]
    InvalidPayload { source: serde_json::Error },
}
