//! Anthropic model specs and provider-owned custom defaults.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable identity of the Anthropic provider.
pub const PROVIDER_ID: ProviderId = ProviderId::new("anthropic");

const THINKING_EFFORTS: &[Effort] = &[
    Effort::Low,
    Effort::Medium,
    Effort::High,
    Effort::ExtraHigh,
    Effort::Max,
];

/// Controls used by an unlisted Anthropic model unless the caller replaces them.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: 64_000,
    efforts: EffortSet::new(Effort::Disabled)
        .with(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High)
        .with(Effort::ExtraHigh)
        .with(Effort::Max),
    default_effort: None,
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: true,
    replay_efforts: THINKING_EFFORTS,
};

const REASONING_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];

const HAIKU_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
];

/// Balanced Claude 5 model with high adaptive thinking by default.
pub const CLAUDE_SONNET_5: ModelSpec = claude_spec(
    "claude-sonnet-5",
    1_000_000,
    128_000,
    EffortSet::new(Effort::Disabled)
        .with(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High),
    Effort::High,
    RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Fast,
        cost: CostTier::High,
    },
    REASONING_FEATURES,
);

/// Claude Opus 5 with low through high and maximum adaptive thinking.
pub const CLAUDE_OPUS_5: ModelSpec = claude_spec(
    "claude-opus-5",
    1_000_000,
    128_000,
    EffortSet::new(Effort::Disabled)
        .with(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High)
        .with(Effort::Max),
    Effort::High,
    RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::Premium,
    },
    REASONING_FEATURES,
);

/// Claude Fable 5 for long-running agent work.
pub const CLAUDE_FABLE_5: ModelSpec = claude_spec(
    "claude-fable-5",
    1_000_000,
    128_000,
    EffortSet::new(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High),
    Effort::High,
    RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Slow,
        cost: CostTier::Premium,
    },
    REASONING_FEATURES,
);

/// Previous-generation balanced Claude model.
pub const CLAUDE_SONNET_4_6: ModelSpec = claude_spec(
    "claude-sonnet-4-6",
    200_000,
    128_000,
    EffortSet::new(Effort::Disabled),
    Effort::Disabled,
    RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Medium,
        cost: CostTier::High,
    },
    REASONING_FEATURES,
);

/// Claude Opus 4.7 with optional maximum adaptive thinking.
pub const CLAUDE_OPUS_4_7: ModelSpec = claude_spec(
    "claude-opus-4-7",
    200_000,
    128_000,
    EffortSet::new(Effort::Disabled).with(Effort::Max),
    Effort::Disabled,
    RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Slow,
        cost: CostTier::Premium,
    },
    REASONING_FEATURES,
);

/// Low-latency Claude model for short turns.
pub const CLAUDE_HAIKU_4_5: ModelSpec = claude_spec(
    "claude-haiku-4-5",
    200_000,
    64_000,
    EffortSet::new(Effort::Disabled),
    Effort::Disabled,
    RoutingTiers {
        intelligence: IntelligenceScore::Seven,
        speed: SpeedTier::Fast,
        cost: CostTier::Low,
    },
    HAIKU_FEATURES,
);

/// One spec for each known Anthropic provider model.
pub const CATALOG: &[ModelSpec] = &[
    CLAUDE_SONNET_5,
    CLAUDE_OPUS_5,
    CLAUDE_FABLE_5,
    CLAUDE_SONNET_4_6,
    CLAUDE_OPUS_4_7,
    CLAUDE_HAIKU_4_5,
];

/// Returns Anthropic models known to this provider crate.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}

const fn claude_spec(
    model_id: &'static str,
    context_window_tokens: u32,
    max_output_tokens: u32,
    efforts: EffortSet,
    default_effort: Effort,
    routing: RoutingTiers,
    features: &'static [ModelFeature],
) -> ModelSpec {
    ModelSpec {
        provider: PROVIDER_ID,
        provider_model_id: Cow::Borrowed(model_id),
        context_window_tokens,
        max_output_tokens,
        efforts,
        default_effort: Some(default_effort),
        forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
        sampling: Sampling::FixedWhileThinking,
        supports_stop_sequences: true,
        supports_deferred: false,
        requires_output_limit: true,
        replay_efforts: Cow::Borrowed(THINKING_EFFORTS),
        features: Cow::Borrowed(features),
        routing: Some(routing),
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
