//! Google model specs and provider-owned custom defaults.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, IntelligenceScore,
    ModelFeature, ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable identity of the Google provider.
pub const PROVIDER_ID: ProviderId = ProviderId::new("google");

/// Controls used by an unlisted Google model unless the caller replaces them.
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
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &Effort::ALL,
};

/// Previous-generation flagship provider id for existing-account access.
pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";

/// Previous-generation balanced provider id for existing-account access.
pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";

/// Previous-generation low-latency provider id for existing-account access.
pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite";

const GEMINI_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Vision,
    ModelFeature::VideoInput,
    ModelFeature::LongContext,
    ModelFeature::Reasoning,
];
const IMAGE_FEATURES: &[ModelFeature] = &[ModelFeature::ImageGeneration, ModelFeature::Vision];
const VIDEO_FEATURES: &[ModelFeature] = &[ModelFeature::VideoGeneration, ModelFeature::Vision];

/// Balanced Gemini model with medium and high thinking.
pub const GEMINI_3_6_FLASH: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("gemini-3.6-flash"),
    context_window_tokens: 1_048_576,
    max_output_tokens: 65_536,
    efforts: EffortSet::new(Effort::Medium).with(Effort::High),
    default_effort: Effort::Medium,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&Effort::ALL),
    features: Cow::Borrowed(GEMINI_FEATURES),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Fast,
        cost: CostTier::Medium,
    }),
};

/// Low-latency Gemini model with provider-default minimal thinking.
pub const GEMINI_3_5_FLASH_LITE: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("gemini-3.5-flash-lite"),
    context_window_tokens: 1_048_576,
    max_output_tokens: 65_536,
    efforts: EffortSet::new(Effort::Disabled),
    default_effort: Effort::Disabled,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&Effort::ALL),
    features: Cow::Borrowed(GEMINI_FEATURES),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Seven,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
    }),
};

/// Gemini image generation model.
pub const GEMINI_3_1_FLASH_IMAGE: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("gemini-3.1-flash-image"),
    context_window_tokens: 131_072,
    max_output_tokens: 0,
    efforts: EffortSet::new(Effort::Disabled),
    default_effort: Effort::Disabled,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[]),
    features: Cow::Borrowed(IMAGE_FEATURES),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Nine,
        speed: SpeedTier::Fast,
        cost: CostTier::Medium,
    }),
};

/// Veo video generation preview model.
pub const VEO_3_1_GENERATE_PREVIEW: ModelSpec = ModelSpec {
    provider: PROVIDER_ID,
    provider_model_id: Cow::Borrowed("veo-3.1-generate-preview"),
    context_window_tokens: 1_024,
    max_output_tokens: 0,
    efforts: EffortSet::new(Effort::Disabled),
    default_effort: Effort::Disabled,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: Cow::Borrowed(&[]),
    features: Cow::Borrowed(VIDEO_FEATURES),
    routing: Some(RoutingTiers {
        intelligence: IntelligenceScore::Ten,
        speed: SpeedTier::Slow,
        cost: CostTier::Premium,
    }),
};

/// One spec for each known Google provider model.
pub const CATALOG: &[ModelSpec] = &[
    GEMINI_3_6_FLASH,
    GEMINI_3_5_FLASH_LITE,
    GEMINI_3_1_FLASH_IMAGE,
    VEO_3_1_GENERATE_PREVIEW,
];

/// Returns Google models known to this provider crate.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
