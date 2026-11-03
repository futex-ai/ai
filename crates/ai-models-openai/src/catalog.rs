//! OpenAI model specs and provider-owned custom defaults.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable identity of the OpenAI provider.
pub const PROVIDER_ID: ProviderId = ProviderId::new("openai");

/// Controls used by an unlisted OpenAI model unless the caller replaces them.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: u32::MAX,
    efforts: EffortSet::new(Effort::Disabled)
        .with(Effort::Low)
        .with(Effort::Medium)
        .with(Effort::High)
        .with(Effort::ExtraHigh)
        .with(Effort::Max),
    default_effort: Effort::Disabled,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::FixedWhileThinking,
    supports_stop_sequences: false,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &Effort::ALL,
};

const GPT_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];
const IMAGE_FEATURES: &[ModelFeature] = &[ModelFeature::ImageGeneration, ModelFeature::Vision];
const SOL_EFFORTS: EffortSet = EffortSet::new(Effort::Low)
    .with(Effort::Medium)
    .with(Effort::High)
    .with(Effort::ExtraHigh)
    .with(Effort::Max);
const GPT_5_5_EFFORTS: EffortSet = EffortSet::new(Effort::Low)
    .with(Effort::Medium)
    .with(Effort::High)
    .with(Effort::ExtraHigh);

/// Frontier GPT-5.6 model with medium reasoning by default.
pub const GPT_5_6_SOL: ModelSpec = gpt_spec(
    "gpt-5.6-sol",
    1_050_000,
    SOL_EFFORTS,
    Effort::Medium,
    RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::Premium,
    },
);

/// Balanced GPT-5.6 model.
pub const GPT_5_6_TERRA: ModelSpec = gpt_spec(
    "gpt-5.6-terra",
    1_050_000,
    EffortSet::new(Effort::Low).with(Effort::Medium),
    Effort::Medium,
    RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Fast,
        cost: CostTier::High,
    },
);

/// Low-latency GPT-5.6 model.
pub const GPT_5_6_LUNA: ModelSpec = gpt_spec(
    "gpt-5.6-luna",
    1_050_000,
    EffortSet::new(Effort::Low).with(Effort::Medium),
    Effort::Medium,
    RoutingTiers {
        intelligence: IntelligenceScore::Eight,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Medium,
    },
);

/// Previous-generation flagship with low through extra-high reasoning.
pub const GPT_5_5: ModelSpec = gpt_spec(
    "gpt-5.5",
    400_000,
    GPT_5_5_EFFORTS,
    Effort::Medium,
    RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::Premium,
    },
);

/// Cost-optimized GPT-5.4 model.
pub const GPT_5_4_MINI: ModelSpec = gpt_spec(
    "gpt-5.4-mini",
    400_000,
    EffortSet::new(Effort::Disabled),
    Effort::Disabled,
    RoutingTiers {
        intelligence: IntelligenceScore::Eight,
        speed: SpeedTier::Fast,
        cost: CostTier::Medium,
    },
);

/// Smallest configured GPT-5.4 model.
pub const GPT_5_4_NANO: ModelSpec = gpt_spec(
    "gpt-5.4-nano",
    400_000,
    EffortSet::new(Effort::Disabled),
    Effort::Disabled,
    RoutingTiers {
        intelligence: IntelligenceScore::Six,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
    },
);

/// Image generation and editing model.
pub const GPT_IMAGE_2: ModelSpec = media_spec(
    "gpt-image-2",
    IMAGE_FEATURES,
    RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::Premium,
    },
);

/// One spec for each known OpenAI provider model.
pub const CATALOG: &[ModelSpec] = &[
    GPT_5_6_SOL,
    GPT_5_6_TERRA,
    GPT_5_6_LUNA,
    GPT_5_5,
    GPT_5_4_MINI,
    GPT_5_4_NANO,
    GPT_IMAGE_2,
];

/// Returns OpenAI models known to this provider crate.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}

const fn gpt_spec(
    model_id: &'static str,
    context_window_tokens: u32,
    efforts: EffortSet,
    default_effort: Effort,
    routing: RoutingTiers,
) -> ModelSpec {
    ModelSpec {
        provider: PROVIDER_ID,
        provider_model_id: Cow::Borrowed(model_id),
        context_window_tokens,
        max_output_tokens: 128_000,
        efforts,
        default_effort,
        forced_tool_choice: ForcedToolChoice::Always,
        sampling: Sampling::FixedWhileThinking,
        supports_stop_sequences: false,
        supports_deferred: false,
        requires_output_limit: false,
        replay_efforts: Cow::Borrowed(&Effort::ALL),
        features: Cow::Borrowed(GPT_FEATURES),
        routing: Some(routing),
    }
}

const fn media_spec(
    model_id: &'static str,
    features: &'static [ModelFeature],
    routing: RoutingTiers,
) -> ModelSpec {
    ModelSpec {
        provider: PROVIDER_ID,
        provider_model_id: Cow::Borrowed(model_id),
        context_window_tokens: 0,
        max_output_tokens: 0,
        efforts: EffortSet::new(Effort::Disabled),
        default_effort: Effort::Disabled,
        forced_tool_choice: ForcedToolChoice::Always,
        sampling: Sampling::FixedWhileThinking,
        supports_stop_sequences: false,
        supports_deferred: false,
        requires_output_limit: false,
        replay_efforts: Cow::Borrowed(&[]),
        features: Cow::Borrowed(features),
        routing: Some(routing),
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
