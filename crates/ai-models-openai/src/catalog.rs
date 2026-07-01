//! Known OpenAI model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// OpenAI flagship model id used by default workspace deployments.
pub const GPT_5_5: &str = "gpt-5.5";

/// OpenAI mid-tier model id with a balanced speed/cost profile.
pub const GPT_5_5_MINI: &str = "gpt-5.5-mini";

/// OpenAI low-latency model id for short, cheap turns.
pub const GPT_5_5_NANO: &str = "gpt-5.5-nano";

/// OpenAI flagship model id with explicit low reasoning effort.
pub const GPT_5_5_THINKING_LOW: &str = "gpt-5.5-thinking-low";

/// OpenAI flagship model id with explicit high reasoning effort.
pub const GPT_5_5_THINKING_HIGH: &str = "gpt-5.5-thinking-high";

/// OpenAI flagship model id with explicit extra-high reasoning effort.
pub const GPT_5_5_THINKING_EXTRA_HIGH: &str = "gpt-5.5-thinking-extra-high";

const GPT_5_5_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GPT_5_5_MINI_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

const GPT_5_5_NANO_FEATURES: &[ModelFeature] =
    &[ModelFeature::ToolCalling, ModelFeature::StructuredOutput];

/// Returns OpenAI models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        gpt_5_5_variant(
            GPT_5_5,
            SpeedTier::Medium,
            CostTier::Premium,
            ThinkingLevel::Medium,
        ),
        gpt_5_5_variant(
            GPT_5_5_THINKING_LOW,
            SpeedTier::Fast,
            CostTier::High,
            ThinkingLevel::Low,
        ),
        gpt_5_5_variant(
            GPT_5_5_THINKING_HIGH,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::High,
        ),
        gpt_5_5_variant(
            GPT_5_5_THINKING_EXTRA_HIGH,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::ExtraHigh,
        ),
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: GPT_5_5_MINI,
            provider_model_id: GPT_5_5_MINI,
            context_window_tokens: 400_000,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Fast,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Disabled,
            features: GPT_5_5_MINI_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: GPT_5_5_NANO,
            provider_model_id: GPT_5_5_NANO,
            context_window_tokens: 128_000,
            intelligence_score: IntelligenceScore::Six,
            speed: SpeedTier::VeryFast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: GPT_5_5_NANO_FEATURES,
        },
    ]
}

fn gpt_5_5_variant(
    id: &'static str,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::OpenAi,
        id,
        provider_model_id: GPT_5_5,
        context_window_tokens: 400_000,
        intelligence_score: IntelligenceScore::Ten,
        speed,
        cost,
        thinking_level,
        features: GPT_5_5_FEATURES,
    }
}
