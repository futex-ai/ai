//! Known OpenAI model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// OpenAI frontier model id used by default workspace deployments.
pub const GPT_5_6_SOL: &str = "gpt-5.6-sol";

/// OpenAI balanced GPT-5.6 model id.
pub const GPT_5_6_TERRA: &str = "gpt-5.6-terra";

/// OpenAI low-latency GPT-5.6 model id.
pub const GPT_5_6_LUNA: &str = "gpt-5.6-luna";

/// OpenAI GPT-5.6 Sol model id with explicit low reasoning effort.
pub const GPT_5_6_SOL_THINKING_LOW: &str = "gpt-5.6-sol-thinking-low";

/// OpenAI GPT-5.6 Sol model id with explicit high reasoning effort.
pub const GPT_5_6_SOL_THINKING_HIGH: &str = "gpt-5.6-sol-thinking-high";

/// OpenAI GPT-5.6 Sol model id with explicit extra-high reasoning effort.
pub const GPT_5_6_SOL_THINKING_EXTRA_HIGH: &str = "gpt-5.6-sol-thinking-extra-high";

/// OpenAI GPT-5.6 Sol model id with maximum reasoning effort.
pub const GPT_5_6_SOL_THINKING_MAX: &str = "gpt-5.6-sol-thinking-max";

/// Previous-generation OpenAI flagship model id.
pub const GPT_5_5: &str = "gpt-5.5";

/// OpenAI mid-tier model id with a balanced speed/cost profile.
pub const GPT_5_4_MINI: &str = "gpt-5.4-mini";

/// OpenAI low-latency model id for short, cheap turns.
pub const GPT_5_4_NANO: &str = "gpt-5.4-nano";

/// OpenAI flagship model id with explicit low reasoning effort.
pub const GPT_5_5_THINKING_LOW: &str = "gpt-5.5-thinking-low";

/// OpenAI flagship model id with explicit high reasoning effort.
pub const GPT_5_5_THINKING_HIGH: &str = "gpt-5.5-thinking-high";

/// OpenAI flagship model id with explicit extra-high reasoning effort.
pub const GPT_5_5_THINKING_EXTRA_HIGH: &str = "gpt-5.5-thinking-extra-high";

/// Current OpenAI image generation and editing model id.
pub const GPT_IMAGE_2: &str = "gpt-image-2";

/// Current OpenAI video generation model id.
pub const SORA_2: &str = "sora-2";

const GPT_5_6_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GPT_5_5_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GPT_5_4_EFFICIENT_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const GPT_IMAGE_2_FEATURES: &[ModelFeature] =
    &[ModelFeature::ImageGeneration, ModelFeature::Vision];

const SORA_2_FEATURES: &[ModelFeature] = &[ModelFeature::VideoGeneration, ModelFeature::Vision];

/// Returns OpenAI models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        gpt_5_6_sol_variant(
            GPT_5_6_SOL,
            SpeedTier::Medium,
            CostTier::Premium,
            ThinkingLevel::Medium,
        ),
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: GPT_5_6_TERRA,
            provider_model_id: GPT_5_6_TERRA,
            context_window_tokens: 1_050_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Fast,
            cost: CostTier::High,
            thinking_level: ThinkingLevel::Medium,
            features: GPT_5_6_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: GPT_5_6_LUNA,
            provider_model_id: GPT_5_6_LUNA,
            context_window_tokens: 1_050_000,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::VeryFast,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Medium,
            features: GPT_5_6_FEATURES,
        },
        gpt_5_6_sol_variant(
            GPT_5_6_SOL_THINKING_LOW,
            SpeedTier::Fast,
            CostTier::High,
            ThinkingLevel::Low,
        ),
        gpt_5_6_sol_variant(
            GPT_5_6_SOL_THINKING_HIGH,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::High,
        ),
        gpt_5_6_sol_variant(
            GPT_5_6_SOL_THINKING_EXTRA_HIGH,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::ExtraHigh,
        ),
        gpt_5_6_sol_variant(
            GPT_5_6_SOL_THINKING_MAX,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::Max,
        ),
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
            id: GPT_5_4_MINI,
            provider_model_id: GPT_5_4_MINI,
            context_window_tokens: 400_000,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Fast,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Disabled,
            features: GPT_5_4_EFFICIENT_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: GPT_5_4_NANO,
            provider_model_id: GPT_5_4_NANO,
            context_window_tokens: 400_000,
            intelligence_score: IntelligenceScore::Six,
            speed: SpeedTier::VeryFast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: GPT_5_4_EFFICIENT_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: GPT_IMAGE_2,
            provider_model_id: GPT_IMAGE_2,
            context_window_tokens: 0,
            intelligence_score: IntelligenceScore::Ten,
            speed: SpeedTier::Medium,
            cost: CostTier::Premium,
            thinking_level: ThinkingLevel::Disabled,
            features: GPT_IMAGE_2_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: SORA_2,
            provider_model_id: SORA_2,
            context_window_tokens: 0,
            intelligence_score: IntelligenceScore::Ten,
            speed: SpeedTier::Slow,
            cost: CostTier::Premium,
            thinking_level: ThinkingLevel::Disabled,
            features: SORA_2_FEATURES,
        },
    ]
}

fn gpt_5_6_sol_variant(
    id: &'static str,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::OpenAi,
        id,
        provider_model_id: GPT_5_6_SOL,
        context_window_tokens: 1_050_000,
        intelligence_score: IntelligenceScore::Ten,
        speed,
        cost,
        thinking_level,
        features: GPT_5_6_FEATURES,
    }
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

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
