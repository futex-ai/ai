//! Model specs and custom defaults owned by the kimi dialect.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable provider identity for configuration and replay state.
pub const PROVIDER_ID: ProviderId = ProviderId::new("kimi");

/// Defaults for an unlisted model, without routing features or scores.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: 131_072,
    efforts: EffortSet::new(Effort::Low)
        .with(Effort::High)
        .with(Effort::Max),
    default_effort: Effort::Max,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Fixed,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[Effort::Low, Effort::High, Effort::Max],
};

/// kimi-k3 model facts at its default max effort.
pub const KIMI_K3: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("kimi-k3"),
    context_window_tokens: 1_000_000,
    max_output_tokens: 131_072,
    efforts: EffortSet::new(Effort::Low)
        .with(Effort::High)
        .with(Effort::Max),
    default_effort: Effort::Max,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Fixed,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::Low, Effort::High, Effort::Max]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Slow,
        cost: CostTier::Premium,
    }),
};

/// One model spec for each provider model id.
pub const CATALOG: &[ModelSpec] = &[KIMI_K3];

/// Returns the model specs known to this dialect.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}
