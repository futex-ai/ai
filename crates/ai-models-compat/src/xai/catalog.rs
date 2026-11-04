//! Model specs and custom defaults owned by the xai dialect.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable provider identity for configuration and replay state.
pub const PROVIDER_ID: ProviderId = ProviderId::new("xai");

/// Defaults for an unlisted model, without routing features or scores.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: u32::MAX,
    efforts: EffortSet::new(Effort::Disabled)
        .with(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High)
        .with(Effort::ExtraHigh)
        .with(Effort::Max),
    default_effort: None,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: true,
    requires_output_limit: false,
    replay_efforts: &[],
};

/// grok-4.5 model facts at its default high effort.
pub const GROK_4_5: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("grok-4.5"),
    context_window_tokens: 500_000,
    max_output_tokens: 500_000,
    efforts: EffortSet::new(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High),
    default_effort: Some(Effort::High),
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: true,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::High,
    }),
};

/// grok-4.20-reasoning model facts at its default high effort.
pub const GROK_4_20_REASONING: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("grok-4.20-reasoning"),
    context_window_tokens: 1_000_000,
    max_output_tokens: 1_000_000,
    efforts: EffortSet::new(Effort::High),
    default_effort: Some(Effort::High),
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: true,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Eight,
        speed: SpeedTier::Medium,
        cost: CostTier::High,
    }),
};

/// grok-4.20 model facts without an explicit effort control.
pub const GROK_4_20: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("grok-4.20"),
    context_window_tokens: 1_000_000,
    max_output_tokens: 1_000_000,
    efforts: EffortSet::empty(),
    default_effort: None,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: true,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Seven,
        speed: SpeedTier::Medium,
        cost: CostTier::Medium,
    }),
};

/// One model spec for each provider model id.
pub const CATALOG: &[ModelSpec] = &[GROK_4_5, GROK_4_20_REASONING, GROK_4_20];

/// Returns the model specs known to this dialect.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}
