//! Model specs and custom defaults owned by the qwen dialect.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable provider identity for configuration and replay state.
pub const PROVIDER_ID: ProviderId = ProviderId::new("qwen");

pub(super) const QWEN_3_7_MAX_ID: &str = "qwen3.7-max";
pub(super) const QWEN_3_7_PLUS_ID: &str = "qwen3.7-plus";
pub(super) const QWEN_3_7_FLASH_ID: &str = "qwen3.7-flash";

/// Defaults for an unlisted model, without routing features or scores.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: 65_536,
    efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
    default_effort: Effort::High,
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[Effort::High],
};

/// qwen3.7-max model facts at its default high effort.
pub const QWEN_3_7_MAX: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed(QWEN_3_7_MAX_ID),
    context_window_tokens: 1_000_000,
    max_output_tokens: 65_536,
    efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
    default_effort: Effort::High,
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::High]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Slow,
        cost: CostTier::Premium,
    }),
};

/// qwen3.7-plus model facts at its default high effort.
pub const QWEN_3_7_PLUS: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed(QWEN_3_7_PLUS_ID),
    context_window_tokens: 1_000_000,
    max_output_tokens: 65_536,
    efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
    default_effort: Effort::High,
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::High]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Medium,
        cost: CostTier::Medium,
    }),
};

/// qwen3.7-flash model facts at its default high effort.
pub const QWEN_3_7_FLASH: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed(QWEN_3_7_FLASH_ID),
    context_window_tokens: 1_000_000,
    max_output_tokens: 65_536,
    efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
    default_effort: Effort::High,
    forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[Effort::High]),
    features: Cow::Borrowed(&[
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ]),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Eight,
        speed: SpeedTier::Fast,
        cost: CostTier::Low,
    }),
};

/// One model spec for each provider model id.
pub const CATALOG: &[ModelSpec] = &[QWEN_3_7_MAX, QWEN_3_7_PLUS, QWEN_3_7_FLASH];

/// Returns the model specs known to this dialect.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}
