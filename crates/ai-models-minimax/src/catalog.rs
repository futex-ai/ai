//! Known MiniMax model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// MiniMax M3 catalog and provider model id.
pub const MINIMAX_M3: &str = "MiniMax-M3";

/// MiniMax M3 catalog variant with thinking disabled.
pub const MINIMAX_M3_THINKING_DISABLED: &str = "MiniMax-M3-thinking-disabled";

/// MiniMax M2.7 catalog and provider model id.
pub const MINIMAX_M2_7: &str = "MiniMax-M2.7";

/// MiniMax M2.7 high-speed catalog and provider model id.
pub const MINIMAX_M2_7_HIGHSPEED: &str = "MiniMax-M2.7-highspeed";

const M3_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const M3_DISABLED_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

const M2_7_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

/// Returns current non-legacy MiniMax models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        KnownModelSpec {
            provider: ProviderKind::MiniMax,
            id: MINIMAX_M3,
            provider_model_id: MINIMAX_M3,
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Medium,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Medium,
            features: M3_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::MiniMax,
            id: MINIMAX_M3_THINKING_DISABLED,
            provider_model_id: MINIMAX_M3,
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Fast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: M3_DISABLED_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::MiniMax,
            id: MINIMAX_M2_7,
            provider_model_id: MINIMAX_M2_7,
            context_window_tokens: 204_800,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Medium,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Medium,
            features: M2_7_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::MiniMax,
            id: MINIMAX_M2_7_HIGHSPEED,
            provider_model_id: MINIMAX_M2_7_HIGHSPEED,
            context_window_tokens: 204_800,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Fast,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Medium,
            features: M2_7_FEATURES,
        },
    ]
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
