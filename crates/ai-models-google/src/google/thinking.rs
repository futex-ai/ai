//! Gemini thinking-control mapping.

use ai_interface::Effort;
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
    thinking_level: Effort,
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

fn thinking_level_for_gemini_3(thinking_level: Effort) -> Option<&'static str> {
    match thinking_level {
        Effort::Disabled => None,
        Effort::Low => Some("low"),
        Effort::Medium => Some("medium"),
        Effort::High | Effort::ExtraHigh | Effort::Max => Some("high"),
    }
}

fn thinking_budget(thinking_level: Effort) -> Option<i32> {
    match thinking_level {
        Effort::Disabled => None,
        Effort::Low => Some(1024),
        Effort::Medium => Some(4096),
        Effort::High => Some(8192),
        Effort::ExtraHigh => Some(16_384),
        Effort::Max => Some(32_768),
    }
}
