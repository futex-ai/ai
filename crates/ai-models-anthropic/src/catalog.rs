//! Known Anthropic model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// Anthropic balanced model id used by default workspace deployments.
pub const CLAUDE_SONNET_4_6: &str = "claude-sonnet-4-6";

/// Anthropic flagship model id with the highest intelligence tier.
pub const CLAUDE_OPUS_4_7: &str = "claude-opus-4-7";

/// Anthropic low-latency model id for cheap, fast turns.
pub const CLAUDE_HAIKU_4_5: &str = "claude-haiku-4-5";

/// Anthropic flagship model id with maximum adaptive thinking.
pub const CLAUDE_OPUS_4_7_THINKING_MAX: &str = "claude-opus-4-7-thinking-max";

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
