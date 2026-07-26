//! Known xAI model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// xAI frontier model id used by default workspace deployments.
pub const GROK_4_5: &str = "grok-4.5";

/// xAI Grok 4.5 model id with low reasoning effort.
pub const GROK_4_5_THINKING_LOW: &str = "grok-4.5-thinking-low";

/// xAI Grok 4.5 model id with medium reasoning effort.
pub const GROK_4_5_THINKING_MEDIUM: &str = "grok-4.5-thinking-medium";

/// Previous-generation xAI reasoning model id.
pub const GROK_4_20_REASONING: &str = "grok-4.20-reasoning";

/// xAI general-purpose model id without the dedicated reasoning track.
pub const GROK_4_20: &str = "grok-4.20";

/// xAI low-latency model id for short, cheap turns.
pub const GROK_4_20_MINI: &str = "grok-4.20-mini";

/// xAI general-purpose model id with high reasoning effort.
pub const GROK_4_20_THINKING_HIGH: &str = "grok-4.20-thinking-high";

const GROK_4_5_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GROK_4_20_REASONING_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GROK_4_20_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

const GROK_4_20_MINI_FEATURES: &[ModelFeature] =
    &[ModelFeature::ToolCalling, ModelFeature::StructuredOutput];

/// Returns xAI models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        grok_4_5_variant(
            GROK_4_5,
            SpeedTier::Medium,
            CostTier::High,
            ThinkingLevel::High,
        ),
        grok_4_5_variant(
            GROK_4_5_THINKING_LOW,
            SpeedTier::Fast,
            CostTier::Medium,
            ThinkingLevel::Low,
        ),
        grok_4_5_variant(
            GROK_4_5_THINKING_MEDIUM,
            SpeedTier::Medium,
            CostTier::High,
            ThinkingLevel::Medium,
        ),
        KnownModelSpec {
            provider: ProviderKind::Xai,
            id: GROK_4_20_REASONING,
            provider_model_id: GROK_4_20_REASONING,
            context_window_tokens: 256_000,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Medium,
            cost: CostTier::High,
            thinking_level: ThinkingLevel::High,
            features: GROK_4_20_REASONING_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Xai,
            id: GROK_4_20,
            provider_model_id: GROK_4_20,
            context_window_tokens: 256_000,
            intelligence_score: IntelligenceScore::Seven,
            speed: SpeedTier::Medium,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Disabled,
            features: GROK_4_20_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Xai,
            id: GROK_4_20_THINKING_HIGH,
            provider_model_id: GROK_4_20,
            context_window_tokens: 256_000,
            intelligence_score: IntelligenceScore::Seven,
            speed: SpeedTier::Slow,
            cost: CostTier::High,
            thinking_level: ThinkingLevel::High,
            features: GROK_4_20_REASONING_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Xai,
            id: GROK_4_20_MINI,
            provider_model_id: GROK_4_20_MINI,
            context_window_tokens: 128_000,
            intelligence_score: IntelligenceScore::Six,
            speed: SpeedTier::Fast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: GROK_4_20_MINI_FEATURES,
        },
    ]
}

fn grok_4_5_variant(
    id: &'static str,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::Xai,
        id,
        provider_model_id: GROK_4_5,
        context_window_tokens: 500_000,
        intelligence_score: IntelligenceScore::Ten,
        speed,
        cost,
        thinking_level,
        features: GROK_4_5_FEATURES,
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
