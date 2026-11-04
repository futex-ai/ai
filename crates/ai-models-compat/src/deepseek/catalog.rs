//! Model specs and custom defaults owned by the deepseek dialect.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable provider identity for configuration and replay state.
pub const PROVIDER_ID: ProviderId = ProviderId::new("deepseek");

/// Defaults for an unlisted model, without routing features or scores.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: 393_216,
    efforts: EffortSet::new(Effort::Disabled)
        .with(Effort::High)
        .with(Effort::Max),
    default_effort: Some(Effort::High),
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[Effort::High, Effort::Max],
};

/// deepseek-v4-pro model facts at its default high effort.
pub const DEEPSEEK_V4_PRO: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("deepseek-v4-pro"),
    context_window_tokens: 1_000_000,
    max_output_tokens: 393_216,
    efforts: EffortSet::new(Effort::Disabled)
        .with(Effort::High)
        .with(Effort::Max),
    default_effort: Some(Effort::High),
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::High, Effort::Max]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::Low,
    }),
};

/// deepseek-v4-flash model facts at its default high effort.
pub const DEEPSEEK_V4_FLASH: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("deepseek-v4-flash"),
    context_window_tokens: 1_000_000,
    max_output_tokens: 393_216,
    efforts: EffortSet::new(Effort::Disabled)
        .with(Effort::High)
        .with(Effort::Max),
    default_effort: Some(Effort::High),
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::High, Effort::Max]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Fast,
        cost: CostTier::Low,
    }),
};

/// One model spec for each provider model id.
pub const CATALOG: &[ModelSpec] = &[DEEPSEEK_V4_PRO, DEEPSEEK_V4_FLASH];

/// Returns the model specs known to this dialect.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}
