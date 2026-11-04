//! Serialization of the native control selected by the shared Google mapping.

use ai_interface::{Effort, ModelControl, ModelError, ModelResult};
use serde::Serialize;

use crate::PROVIDER_ID;
use crate::effort_mapping::{ThinkingControl, thinking_control};

#[derive(Debug, Serialize)]
pub(super) struct GoogleThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingBudget")]
    thinking_budget: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingLevel")]
    thinking_level: Option<&'static str>,
}

pub(super) fn thinking_config(
    model_id: &str,
    effort: Option<Effort>,
) -> ModelResult<Option<GoogleThinkingConfig>> {
    let Some(effort) = effort else {
        return Ok(None);
    };
    let control = thinking_control(model_id, effort).ok_or_else(|| {
        ModelError::unsupported_control(PROVIDER_ID.as_str(), model_id, ModelControl::Effort)
    })?;
    Ok(Some(match control {
        ThinkingControl::Budget(budget) => GoogleThinkingConfig {
            thinking_budget: Some(budget),
            thinking_level: None,
        },
        ThinkingControl::Level(level) => GoogleThinkingConfig {
            thinking_budget: None,
            thinking_level: Some(level),
        },
    }))
}
