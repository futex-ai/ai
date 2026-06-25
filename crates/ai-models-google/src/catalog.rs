//! Known Google model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Google flagship model id used by default workspace deployments.
pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";

/// Google mid-tier model id with a balanced speed/cost profile.
pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";

/// Google low-latency model id for short, cheap turns.
pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite";

/// Google Pro model id with explicit high thinking budget.
pub const GEMINI_2_5_PRO_THINKING_HIGH: &str = "gemini-2.5-pro-thinking-high";

/// Google Pro model id with the maximum supported thinking budget.
pub const GEMINI_2_5_PRO_THINKING_MAX: &str = "gemini-2.5-pro-thinking-max";

const GEMINI_2_5_PRO_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GEMINI_2_5_FLASH_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

const GEMINI_2_5_FLASH_LITE_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::LongContext,
];

/// Returns Google models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        gemini_2_5_pro_variant(
            GEMINI_2_5_PRO,
            SpeedTier::Medium,
            CostTier::Medium,
            ThinkingLevel::Disabled,
        ),
        gemini_2_5_pro_variant(
            GEMINI_2_5_PRO_THINKING_HIGH,
            SpeedTier::Medium,
            CostTier::High,
            ThinkingLevel::High,
        ),
        gemini_2_5_pro_variant(
            GEMINI_2_5_PRO_THINKING_MAX,
            SpeedTier::Slow,
            CostTier::High,
            ThinkingLevel::Max,
        ),
        KnownModelSpec {
            provider: ProviderKind::Google,
            id: GEMINI_2_5_FLASH,
            provider_model_id: GEMINI_2_5_FLASH,
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Fast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: GEMINI_2_5_FLASH_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Google,
            id: GEMINI_2_5_FLASH_LITE,
            provider_model_id: GEMINI_2_5_FLASH_LITE,
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Six,
            speed: SpeedTier::VeryFast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: GEMINI_2_5_FLASH_LITE_FEATURES,
        },
    ]
}

fn gemini_2_5_pro_variant(
    id: &'static str,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::Google,
        id,
        provider_model_id: GEMINI_2_5_PRO,
        context_window_tokens: 1_000_000,
        intelligence_score: IntelligenceScore::Nine,
        speed,
        cost,
        thinking_level,
        features: GEMINI_2_5_PRO_FEATURES,
    }
}
