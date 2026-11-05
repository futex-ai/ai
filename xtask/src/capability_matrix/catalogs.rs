//! Composition of provider-owned catalogs and their custom defaults.

use ai_interface::{MOCK_CATALOG, MOCK_CUSTOM_DEFAULTS, MOCK_PROVIDER, ModelSpec};
use ai_models_compat::{deepseek, kimi, minimax, qwen, xai};

pub(super) fn model_specs() -> Vec<ModelSpec> {
    let providers = [
        (
            ai_models_anthropic::CATALOG,
            ai_models_anthropic::PROVIDER_ID,
            ai_models_anthropic::CUSTOM_DEFAULTS,
        ),
        (
            deepseek::CATALOG,
            deepseek::PROVIDER_ID,
            deepseek::CUSTOM_DEFAULTS,
        ),
        (
            ai_models_google::CATALOG,
            ai_models_google::PROVIDER_ID,
            ai_models_google::CUSTOM_DEFAULTS,
        ),
        (kimi::CATALOG, kimi::PROVIDER_ID, kimi::CUSTOM_DEFAULTS),
        (
            minimax::CATALOG,
            minimax::PROVIDER_ID,
            minimax::CUSTOM_DEFAULTS,
        ),
        (MOCK_CATALOG, MOCK_PROVIDER, MOCK_CUSTOM_DEFAULTS),
        (
            ai_models_openai::CATALOG,
            ai_models_openai::PROVIDER_ID,
            ai_models_openai::CUSTOM_DEFAULTS,
        ),
        (qwen::CATALOG, qwen::PROVIDER_ID, qwen::CUSTOM_DEFAULTS),
        (
            ai_models_typesafe::CATALOG,
            ai_models_typesafe::PROVIDER_ID,
            ai_models_typesafe::CUSTOM_DEFAULTS,
        ),
        (xai::CATALOG, xai::PROVIDER_ID, xai::CUSTOM_DEFAULTS),
    ];
    let mut specs = Vec::new();
    for (catalog, provider, defaults) in providers {
        specs.extend_from_slice(catalog);
        specs.push(ModelSpec::custom(provider, "custom", &defaults));
    }
    specs
}
