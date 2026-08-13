//! Known Anthropic model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Anthropic balanced model id used by default workspace deployments.
pub const CLAUDE_SONNET_5: &str = "claude-sonnet-5";

/// Anthropic model id for complex agentic and enterprise work.
pub const CLAUDE_OPUS_5: &str = "claude-opus-5";

/// Anthropic model id for the most demanding long-running agents.
pub const CLAUDE_FABLE_5: &str = "claude-fable-5";

/// Anthropic Opus 5 model id with maximum adaptive thinking.
pub const CLAUDE_OPUS_5_THINKING_MAX: &str = "claude-opus-5-thinking-max";

/// Previous-generation Anthropic balanced model id.
pub const CLAUDE_SONNET_4_6: &str = "claude-sonnet-4-6";

/// Anthropic flagship model id with the highest intelligence tier.
pub const CLAUDE_OPUS_4_7: &str = "claude-opus-4-7";

/// Anthropic low-latency model id for cheap, fast turns.
pub const CLAUDE_HAIKU_4_5: &str = "claude-haiku-4-5";

/// Anthropic flagship model id with maximum adaptive thinking.
pub const CLAUDE_OPUS_4_7_THINKING_MAX: &str = "claude-opus-4-7-thinking-max";

const CLAUDE_5_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const CLAUDE_SONNET_4_6_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const CLAUDE_OPUS_4_7_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const CLAUDE_HAIKU_4_5_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

/// Returns Anthropic models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![
        claude_5(
            CLAUDE_SONNET_5,
            CLAUDE_SONNET_5,
            IntelligenceScore::Nine,
            SpeedTier::Fast,
            CostTier::High,
            ThinkingLevel::High,
        ),
        claude_5(
            CLAUDE_OPUS_5,
            CLAUDE_OPUS_5,
            IntelligenceScore::Ten,
            SpeedTier::Medium,
            CostTier::Premium,
            ThinkingLevel::High,
        ),
        claude_5(
            CLAUDE_OPUS_5_THINKING_MAX,
            CLAUDE_OPUS_5,
            IntelligenceScore::Ten,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::Max,
        ),
        claude_5(
            CLAUDE_FABLE_5,
            CLAUDE_FABLE_5,
            IntelligenceScore::Ten,
            SpeedTier::Slow,
            CostTier::Premium,
            ThinkingLevel::High,
        ),
        KnownModelSpec {
            provider: ProviderKind::Anthropic,
            id: CLAUDE_SONNET_4_6,
            provider_model_id: CLAUDE_SONNET_4_6,
            context_window_tokens: 200_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Medium,
            cost: CostTier::High,
            thinking_level: ThinkingLevel::Disabled,
            features: CLAUDE_SONNET_4_6_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Anthropic,
            id: CLAUDE_OPUS_4_7,
            provider_model_id: CLAUDE_OPUS_4_7,
            context_window_tokens: 200_000,
            intelligence_score: IntelligenceScore::Ten,
            speed: SpeedTier::Slow,
            cost: CostTier::Premium,
            thinking_level: ThinkingLevel::Disabled,
            features: CLAUDE_OPUS_4_7_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Anthropic,
            id: CLAUDE_OPUS_4_7_THINKING_MAX,
            provider_model_id: CLAUDE_OPUS_4_7,
            context_window_tokens: 200_000,
            intelligence_score: IntelligenceScore::Ten,
            speed: SpeedTier::Slow,
            cost: CostTier::Premium,
            thinking_level: ThinkingLevel::Max,
            features: CLAUDE_OPUS_4_7_FEATURES,
        },
        KnownModelSpec {
            provider: ProviderKind::Anthropic,
            id: CLAUDE_HAIKU_4_5,
            provider_model_id: CLAUDE_HAIKU_4_5,
            context_window_tokens: 200_000,
            intelligence_score: IntelligenceScore::Seven,
            speed: SpeedTier::Fast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: CLAUDE_HAIKU_4_5_FEATURES,
        },
    ]
}

pub(crate) fn find_known_model(model_id: &str) -> Option<(&'static str, ThinkingLevel)> {
    known_models()
        .into_iter()
        .find(|model| model.id == model_id)
        .map(|model| (model.provider_model_id, model.thinking_level))
}

fn claude_5(
    id: &'static str,
    provider_model_id: &'static str,
    intelligence_score: IntelligenceScore,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::Anthropic,
        id,
        provider_model_id,
        context_window_tokens: 1_000_000,
        intelligence_score,
        speed,
        cost,
        thinking_level,
        features: CLAUDE_5_FEATURES,
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
