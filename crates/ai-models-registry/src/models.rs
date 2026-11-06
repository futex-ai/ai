//! Feature-selected conversational model factories.

#[cfg(any(
    feature = "anthropic",
    feature = "deepseek",
    feature = "google",
    feature = "kimi",
    feature = "minimax",
    feature = "openai",
    feature = "qwen",
    feature = "xai"
))]
use std::sync::Arc;

use ai_interface::{DynModel, ModelSpec};
use json_http::DynJsonHttpClient;

use crate::{BuildError, BuildResult, Modality, ProviderConfig};

/// Builds the model adapter of an included, configured provider.
pub(crate) fn build(
    http: DynJsonHttpClient,
    spec: &ModelSpec,
    config: ProviderConfig,
) -> BuildResult<DynModel> {
    match (spec.provider.as_str(), http, config) {
        #[cfg(feature = "anthropic")]
        ("anthropic", http, config) => Ok(Arc::new(
            ai_models_anthropic::AnthropicModel::from_config(http, spec.clone(), config),
        )),
        #[cfg(feature = "deepseek")]
        ("deepseek", http, config) => Ok(Arc::new(
            ai_models_compat::deepseek::DeepSeekModel::from_config(http, spec.clone(), config),
        )),
        #[cfg(feature = "google")]
        ("google", http, config) => Ok(Arc::new(ai_models_google::GoogleModel::from_config(
            http,
            spec.clone(),
            config,
        ))),
        #[cfg(feature = "kimi")]
        ("kimi", http, config) => Ok(Arc::new(ai_models_compat::kimi::KimiModel::from_config(
            http,
            spec.clone(),
            config,
        ))),
        #[cfg(feature = "minimax")]
        ("minimax", http, config) => Ok(Arc::new(
            ai_models_compat::minimax::MiniMaxModel::from_config(http, spec.clone(), config),
        )),
        #[cfg(feature = "openai")]
        ("openai", http, config) => Ok(Arc::new(ai_models_openai::OpenAiModel::from_config(
            http,
            spec.clone(),
            config,
        ))),
        #[cfg(feature = "qwen")]
        ("qwen", http, config) => Ok(Arc::new(ai_models_compat::qwen::QwenModel::from_config(
            http,
            spec.clone(),
            config,
        ))),
        #[cfg(feature = "xai")]
        ("xai", http, config) => Ok(Arc::new(ai_models_compat::xai::XaiModel::from_config(
            http,
            spec.clone(),
            config,
        ))),
        _ => Err(BuildError::UnsupportedModality {
            provider: spec.provider.clone(),
            modality: Modality::Model,
        }),
    }
}
