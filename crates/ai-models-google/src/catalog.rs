//! Known Google model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Google balanced model id used by default workspace deployments.
pub const GEMINI_3_6_FLASH: &str = "gemini-3.6-flash";

/// Google Gemini 3.6 Flash model id with high thinking.
pub const GEMINI_3_6_FLASH_THINKING_HIGH: &str = "gemini-3.6-flash-thinking-high";

/// Google low-latency Gemini 3.5 model id.
pub const GEMINI_3_5_FLASH_LITE: &str = "gemini-3.5-flash-lite";

/// Google balanced image generation model id.
pub const GEMINI_3_1_FLASH_IMAGE: &str = "gemini-3.1-flash-image";

/// Previous-generation Google flagship model id for existing-account access.
pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";

/// Previous-generation Google balanced model id for existing-account access.
pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";

/// Previous-generation Google low-latency model id for existing-account access.
pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite";

/// Logical Gemini 2.5 Pro id with a high thinking budget.
pub const GEMINI_2_5_PRO_THINKING_HIGH: &str = "gemini-2.5-pro-thinking-high";

/// Logical Gemini 2.5 Pro id with the maximum supported thinking budget.
pub const GEMINI_2_5_PRO_THINKING_MAX: &str = "gemini-2.5-pro-thinking-max";

const GEMINI_3_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::VideoInput,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GEMINI_3_1_FLASH_IMAGE_FEATURES: &[ModelFeature] =
    &[ModelFeature::ImageGeneration, ModelFeature::Vision];

/// Returns Google models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        gemini_3_6_flash_variant(
            GEMINI_3_6_FLASH,
            SpeedTier::Fast,
            CostTier::Medium,
            ThinkingLevel::Medium,
        ),
        gemini_3_6_flash_variant(
            GEMINI_3_6_FLASH_THINKING_HIGH,
            SpeedTier::Medium,
            CostTier::High,
            ThinkingLevel::High,
        ),
        KnownModelSpec {
            provider: ProviderKind::Google,
            id: GEMINI_3_5_FLASH_LITE,
            provider_model_id: GEMINI_3_5_FLASH_LITE,
            context_window_tokens: 1_048_576,
            intelligence_score: IntelligenceScore::Seven,
            speed: SpeedTier::VeryFast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: GEMINI_3_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Google,
            id: GEMINI_3_1_FLASH_IMAGE,
            provider_model_id: GEMINI_3_1_FLASH_IMAGE,
            context_window_tokens: 131_072,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Fast,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Disabled,
            features: GEMINI_3_1_FLASH_IMAGE_FEATURES,
        },
    ]
}

fn gemini_3_6_flash_variant(
    id: &'static str,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::Google,
        id,
        provider_model_id: GEMINI_3_6_FLASH,
        context_window_tokens: 1_048_576,
        intelligence_score: IntelligenceScore::Nine,
        speed,
        cost,
        thinking_level,
        features: GEMINI_3_FEATURES,
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
