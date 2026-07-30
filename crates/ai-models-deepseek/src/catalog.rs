//! Known DeepSeek model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Default high-thinking DeepSeek V4 Pro catalog id.
pub const DEEPSEEK_V4_PRO: &str = "deepseek-v4-pro";

/// DeepSeek V4 Pro catalog id with maximum reasoning effort.
pub const DEEPSEEK_V4_PRO_THINKING_MAX: &str = "deepseek-v4-pro-thinking-max";

/// DeepSeek V4 Pro catalog id with provider thinking disabled.
pub const DEEPSEEK_V4_PRO_THINKING_DISABLED: &str = "deepseek-v4-pro-thinking-disabled";

/// Default high-thinking DeepSeek V4 Flash catalog id.
pub const DEEPSEEK_V4_FLASH: &str = "deepseek-v4-flash";

/// DeepSeek V4 Flash catalog id with maximum reasoning effort.
pub const DEEPSEEK_V4_FLASH_THINKING_MAX: &str = "deepseek-v4-flash-thinking-max";

/// DeepSeek V4 Flash catalog id with provider thinking disabled.
pub const DEEPSEEK_V4_FLASH_THINKING_DISABLED: &str = "deepseek-v4-flash-thinking-disabled";

const THINKING_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const DISABLED_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::LongContext,
];

/// Returns current DeepSeek V4 models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        model_spec(
            DEEPSEEK_V4_PRO,
            DEEPSEEK_V4_PRO,
            IntelligenceScore::Ten,
            SpeedTier::Medium,
            ThinkingLevel::High,
        ),
        model_spec(
            DEEPSEEK_V4_PRO_THINKING_MAX,
            DEEPSEEK_V4_PRO,
            IntelligenceScore::Ten,
            SpeedTier::Slow,
            ThinkingLevel::Max,
        ),
        model_spec(
            DEEPSEEK_V4_PRO_THINKING_DISABLED,
            DEEPSEEK_V4_PRO,
            IntelligenceScore::Ten,
            SpeedTier::Fast,
            ThinkingLevel::Disabled,
        ),
        model_spec(
            DEEPSEEK_V4_FLASH,
            DEEPSEEK_V4_FLASH,
            IntelligenceScore::Nine,
            SpeedTier::Fast,
            ThinkingLevel::High,
        ),
        model_spec(
            DEEPSEEK_V4_FLASH_THINKING_MAX,
            DEEPSEEK_V4_FLASH,
            IntelligenceScore::Nine,
            SpeedTier::Medium,
            ThinkingLevel::Max,
        ),
        model_spec(
            DEEPSEEK_V4_FLASH_THINKING_DISABLED,
            DEEPSEEK_V4_FLASH,
            IntelligenceScore::Nine,
            SpeedTier::VeryFast,
            ThinkingLevel::Disabled,
        ),
    ]
}

fn model_spec(
    id: &'static str,
    provider_model_id: &'static str,
    intelligence_score: IntelligenceScore,
    speed: SpeedTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::DeepSeek,
        id,
        provider_model_id,
        context_window_tokens: 1_000_000,
        intelligence_score,
        speed,
        cost: CostTier::Low,
        thinking_level,
        features: if thinking_level == ThinkingLevel::Disabled {
            DISABLED_FEATURES
        } else {
            THINKING_FEATURES
        },
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
