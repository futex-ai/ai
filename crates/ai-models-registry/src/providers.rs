//! Feature-selected catalogs in stable provider order.

use std::sync::Arc;

use ai_interface::{
    LegacyMessageConverter, LegacyProviderConverter, ModelCatalog, ModelSpec, ProviderId,
};

/// Provider identities whose features are enabled, in catalog order.
pub(crate) const INCLUDED: &[ProviderId] = &[
    #[cfg(feature = "anthropic")]
    ai_models_anthropic::PROVIDER_ID,
    #[cfg(feature = "deepseek")]
    ai_models_compat::deepseek::PROVIDER_ID,
    #[cfg(feature = "google")]
    ai_models_google::PROVIDER_ID,
    #[cfg(feature = "kimi")]
    ai_models_compat::kimi::PROVIDER_ID,
    #[cfg(feature = "minimax")]
    ai_models_compat::minimax::PROVIDER_ID,
    #[cfg(feature = "openai")]
    ai_models_openai::PROVIDER_ID,
    #[cfg(feature = "qwen")]
    ai_models_compat::qwen::PROVIDER_ID,
    #[cfg(feature = "typesafe")]
    ai_models_typesafe::PROVIDER_ID,
    #[cfg(feature = "xai")]
    ai_models_compat::xai::PROVIDER_ID,
];

/// Combines included provider catalogs in alphabetical provider order.
pub fn catalog() -> ModelCatalog {
    let catalogs: &[&[ModelSpec]] = &[
        #[cfg(feature = "anthropic")]
        ai_models_anthropic::CATALOG,
        #[cfg(feature = "deepseek")]
        ai_models_compat::deepseek::CATALOG,
        #[cfg(feature = "google")]
        ai_models_google::CATALOG,
        #[cfg(feature = "kimi")]
        ai_models_compat::kimi::CATALOG,
        #[cfg(feature = "minimax")]
        ai_models_compat::minimax::CATALOG,
        #[cfg(feature = "openai")]
        ai_models_openai::CATALOG,
        #[cfg(feature = "qwen")]
        ai_models_compat::qwen::CATALOG,
        #[cfg(feature = "typesafe")]
        ai_models_typesafe::CATALOG,
        #[cfg(feature = "xai")]
        ai_models_compat::xai::CATALOG,
    ];
    ModelCatalog::new().with_models(catalogs.iter().flat_map(|catalog| catalog.iter().cloned()))
}

/// Composes the converters selected by the enabled provider features.
pub(crate) fn converter() -> LegacyMessageConverter {
    let converters: Vec<Arc<dyn LegacyProviderConverter>> = vec![
        #[cfg(feature = "anthropic")]
        Arc::new(ai_models_anthropic::AnthropicLegacyConverter),
        #[cfg(feature = "deepseek")]
        Arc::new(ai_models_compat::deepseek::LegacyConverter),
        #[cfg(feature = "google")]
        Arc::new(ai_models_google::GoogleLegacyProviderConverter),
        #[cfg(feature = "kimi")]
        Arc::new(ai_models_compat::kimi::LegacyConverter),
        #[cfg(feature = "minimax")]
        Arc::new(ai_models_compat::minimax::LegacyConverter),
        #[cfg(feature = "openai")]
        Arc::new(ai_models_openai::OpenAiLegacyConverter),
        #[cfg(feature = "qwen")]
        Arc::new(ai_models_compat::qwen::LegacyConverter),
        #[cfg(feature = "xai")]
        Arc::new(ai_models_compat::xai::LegacyConverter),
    ];
    LegacyMessageConverter::new(converters)
}
