//! Model facts and custom defaults for the built-in development model.

use std::borrow::Cow;

use crate::{
    CostTier, CustomModelDefaults, EffortSet, ForcedToolChoice, IntelligenceScore, MOCK_PROVIDER,
    ModelFeature, ModelSpec, RoutingTiers, Sampling, SpeedTier,
};

/// Defaults for an unlisted development and test model.
pub const MOCK_CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 128_000,
    max_output_tokens: 128_000,
    efforts: EffortSet::empty(),
    default_effort: None,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[],
};

/// Catalog spec for the built-in development and test model.
pub const MOCK_MODEL: ModelSpec = ModelSpec {
    provider: MOCK_PROVIDER,
    provider_model_id: Cow::Borrowed("mock"),
    context_window_tokens: MOCK_CUSTOM_DEFAULTS.context_window_tokens,
    max_output_tokens: MOCK_CUSTOM_DEFAULTS.max_output_tokens,
    efforts: MOCK_CUSTOM_DEFAULTS.efforts,
    default_effort: MOCK_CUSTOM_DEFAULTS.default_effort,
    forced_tool_choice: MOCK_CUSTOM_DEFAULTS.forced_tool_choice,
    sampling: MOCK_CUSTOM_DEFAULTS.sampling,
    supports_stop_sequences: MOCK_CUSTOM_DEFAULTS.supports_stop_sequences,
    supports_deferred: MOCK_CUSTOM_DEFAULTS.supports_deferred,
    requires_output_limit: MOCK_CUSTOM_DEFAULTS.requires_output_limit,
    replay_efforts: Cow::Borrowed(MOCK_CUSTOM_DEFAULTS.replay_efforts),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Five,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
    }),
};

/// Every built-in development model, with one spec per provider model id.
pub const MOCK_CATALOG: &[ModelSpec] = &[MOCK_MODEL];
