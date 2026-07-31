//! Known QwenCloud model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Default high-thinking Qwen 3.7 Max catalog id.
pub const QWEN_3_7_MAX: &str = "qwen3.7-max";

/// Qwen 3.7 Max catalog id with thinking disabled.
pub const QWEN_3_7_MAX_THINKING_DISABLED: &str = "qwen3.7-max-thinking-disabled";

/// Default high-thinking Qwen 3.7 Plus catalog id.
pub const QWEN_3_7_PLUS: &str = "qwen3.7-plus";

/// Qwen 3.7 Plus catalog id with thinking disabled.
pub const QWEN_3_7_PLUS_THINKING_DISABLED: &str = "qwen3.7-plus-thinking-disabled";

/// Default high-thinking Qwen 3.7 Flash catalog id.
pub const QWEN_3_7_FLASH: &str = "qwen3.7-flash";

/// Qwen 3.7 Flash catalog id with thinking disabled.
pub const QWEN_3_7_FLASH_THINKING_DISABLED: &str = "qwen3.7-flash-thinking-disabled";

const TEXT_THINKING_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const TEXT_DISABLED_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::LongContext,
];

const VISION_THINKING_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const VISION_DISABLED_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

/// Returns current stable Qwen 3.7 models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        model_spec(
            QWEN_3_7_MAX,
            QWEN_3_7_MAX,
            IntelligenceScore::Ten,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::High,
            false,
        ),
        model_spec(
            QWEN_3_7_MAX_THINKING_DISABLED,
            QWEN_3_7_MAX,
            IntelligenceScore::Ten,
            SpeedTier::Medium,
            CostTier::Premium,
            ThinkingLevel::Disabled,
            false,
        ),
        model_spec(
            QWEN_3_7_PLUS,
            QWEN_3_7_PLUS,
            IntelligenceScore::Nine,
            SpeedTier::Medium,
            CostTier::Medium,
            ThinkingLevel::High,
            true,
        ),
        model_spec(
            QWEN_3_7_PLUS_THINKING_DISABLED,
            QWEN_3_7_PLUS,
            IntelligenceScore::Nine,
            SpeedTier::Fast,
            CostTier::Medium,
            ThinkingLevel::Disabled,
            true,
        ),
        model_spec(
            QWEN_3_7_FLASH,
            QWEN_3_7_FLASH,
            IntelligenceScore::Eight,
            SpeedTier::Fast,
            CostTier::Low,
            ThinkingLevel::High,
            true,
        ),
        model_spec(
            QWEN_3_7_FLASH_THINKING_DISABLED,
            QWEN_3_7_FLASH,
            IntelligenceScore::Eight,
            SpeedTier::VeryFast,
            CostTier::Low,
            ThinkingLevel::Disabled,
            true,
        ),
    ]
}

fn model_spec(
    id: &'static str,
    provider_model_id: &'static str,
    intelligence_score: IntelligenceScore,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
    vision: bool,
) -> KnownModelSpec {
    let features = match (vision, thinking_level.is_enabled()) {
        (false, true) => TEXT_THINKING_FEATURES,
        (false, false) => TEXT_DISABLED_FEATURES,
        (true, true) => VISION_THINKING_FEATURES,
        (true, false) => VISION_DISABLED_FEATURES,
    };
    KnownModelSpec {
        provider: ProviderKind::Qwen,
        id,
        provider_model_id,
        context_window_tokens: 1_000_000,
        intelligence_score,
        speed,
        cost,
        thinking_level,
        features,
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
