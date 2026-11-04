//! Model specs and custom defaults owned by the minimax dialect.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable provider identity for configuration and replay state.
pub const PROVIDER_ID: ProviderId = ProviderId::new("minimax");

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
    default_effort: Some(Effort::Medium),
    forced_tool_choice: ForcedToolChoice::Never,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: false,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::ExtraHigh,
        Effort::Max,
    ],
};

/// MiniMax-M3 model facts at its default medium effort.
pub const MINIMAX_M3: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("MiniMax-M3"),
    context_window_tokens: 1_000_000,
    max_output_tokens: 512_000,
    efforts: EffortSet::new(Effort::Disabled).with(Effort::Medium),
    default_effort: Some(Effort::Medium),
    forced_tool_choice: ForcedToolChoice::RequiredOnly,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: false,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::Medium]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::VideoInput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Medium,
        cost: CostTier::Low,
    }),
};

/// MiniMax-M2.7 model facts at its default medium effort.
pub const MINIMAX_M2_7: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("MiniMax-M2.7"),
    context_window_tokens: 204_800,
    max_output_tokens: 131_072,
    efforts: EffortSet::new(Effort::Medium),
    default_effort: Some(Effort::Medium),
    forced_tool_choice: ForcedToolChoice::Never,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: false,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::Medium]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Eight,
        speed: SpeedTier::Medium,
        cost: CostTier::Low,
    }),
};

/// MiniMax-M2.7-highspeed model facts at its default medium effort.
pub const MINIMAX_M2_7_HIGHSPEED: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("MiniMax-M2.7-highspeed"),
    context_window_tokens: 204_800,
    max_output_tokens: 131_072,
    efforts: EffortSet::new(Effort::Medium),
    default_effort: Some(Effort::Medium),
    forced_tool_choice: ForcedToolChoice::Never,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: false,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::Medium]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Eight,
        speed: SpeedTier::Fast,
        cost: CostTier::Medium,
    }),
};

/// One model spec for each provider model id.
pub const CATALOG: &[ModelSpec] = &[MINIMAX_M3, MINIMAX_M2_7, MINIMAX_M2_7_HIGHSPEED];

/// Returns the model specs known to this dialect.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}
