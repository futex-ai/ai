//! Known Kimi model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Default max-reasoning Kimi K3 catalog id.
pub const KIMI_K3: &str = "kimi-k3";

/// Kimi K3 catalog id with high reasoning effort.
pub const KIMI_K3_THINKING_HIGH: &str = "kimi-k3-thinking-high";

/// Kimi K3 catalog id with low reasoning effort.
pub const KIMI_K3_THINKING_LOW: &str = "kimi-k3-thinking-low";

const KIMI_K3_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

/// Returns Kimi models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        kimi_k3_spec(
            KIMI_K3,
            ThinkingLevel::Max,
            SpeedTier::Slow,
            CostTier::Premium,
        ),
        kimi_k3_spec(
            KIMI_K3_THINKING_HIGH,
            ThinkingLevel::High,
            SpeedTier::Medium,
            CostTier::Premium,
        ),
        kimi_k3_spec(
            KIMI_K3_THINKING_LOW,
            ThinkingLevel::Low,
            SpeedTier::Fast,
            CostTier::High,
        ),
    ]
}

fn kimi_k3_spec(
    id: &'static str,
    thinking_level: ThinkingLevel,
    speed: SpeedTier,
    cost: CostTier,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::Kimi,
        id,
        provider_model_id: KIMI_K3,
        context_window_tokens: 1_000_000,
        intelligence_score: IntelligenceScore::Ten,
        speed,
        cost,
        thinking_level,
        features: KIMI_K3_FEATURES,
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
