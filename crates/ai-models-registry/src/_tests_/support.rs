//! Provider feature expectations and unused HTTP boundaries.

use std::sync::Arc;

use ai_interface::{
    CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, ModelSpec, ProviderId, Sampling,
};
use json_http::{DynJsonHttpClient, TransportBackedJsonHttpClient};
use unimock::Unimock;

pub(super) const PROVIDERS: &[(&str, bool, [bool; 5])] = &[
    (
        "anthropic",
        cfg!(feature = "anthropic"),
        [true, false, false, false, false],
    ),
    (
        "deepseek",
        cfg!(feature = "deepseek"),
        [true, false, false, false, false],
    ),
    (
        "google",
        cfg!(feature = "google"),
        [true, true, false, false, true],
    ),
    (
        "kimi",
        cfg!(feature = "kimi"),
        [true, false, false, false, false],
    ),
    (
        "minimax",
        cfg!(feature = "minimax"),
        [true, false, false, false, false],
    ),
    (
        "openai",
        cfg!(feature = "openai"),
        [true, true, true, false, false],
    ),
    (
        "qwen",
        cfg!(feature = "qwen"),
        [true, false, false, false, false],
    ),
    (
        "typesafe",
        cfg!(feature = "typesafe"),
        [false, false, false, true, false],
    ),
    (
        "xai",
        cfg!(feature = "xai"),
        [true, false, false, false, false],
    ),
    ("uninstalled", false, [false; 5]),
];

const DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: u32::MAX,
    efforts: EffortSet::new(Effort::Disabled),
    default_effort: Effort::Disabled,
    forced_tool_choice: ForcedToolChoice::Always,
    sampling: Sampling::Adjustable,
    supports_stop_sequences: true,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[],
};

pub(super) fn unused_http() -> DynJsonHttpClient {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}

pub(super) fn spec(provider: &str) -> ModelSpec {
    ModelSpec::custom(
        ProviderId::from(provider.to_owned()),
        "unlisted-model",
        &DEFAULTS,
    )
}
