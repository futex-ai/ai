//! Gemini thinking-control mapping.

use ai_models_core::ThinkingLevel;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct GoogleThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingBudget")]
    thinking_budget: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingLevel")]
    thinking_level: Option<&'static str>,
}

pub(super) fn thinking_config(
    model_id: &str,
    thinking_level: ThinkingLevel,
) -> Option<GoogleThinkingConfig> {
    if model_id.starts_with("gemini-3") {
        return thinking_level_for_gemini_3(thinking_level).map(|thinking_level| {
            GoogleThinkingConfig {
                thinking_budget: None,
                thinking_level: Some(thinking_level),
            }
        });
    }
    thinking_budget(thinking_level).map(|thinking_budget| GoogleThinkingConfig {
        thinking_budget: Some(thinking_budget),
        thinking_level: None,
    })
}

fn thinking_level_for_gemini_3(thinking_level: ThinkingLevel) -> Option<&'static str> {
    match thinking_level {
        ThinkingLevel::Disabled => None,
        ThinkingLevel::Low => Some("low"),
        ThinkingLevel::Medium => Some("medium"),
        ThinkingLevel::High | ThinkingLevel::ExtraHigh | ThinkingLevel::Max => Some("high"),
    }
}

fn thinking_budget(thinking_level: ThinkingLevel) -> Option<i32> {
    match thinking_level {
        ThinkingLevel::Disabled => None,
        ThinkingLevel::Low => Some(1024),
        ThinkingLevel::Medium => Some(4096),
        ThinkingLevel::High => Some(8192),
        ThinkingLevel::ExtraHigh => Some(16_384),
        ThinkingLevel::Max => Some(32_768),
    }
}
