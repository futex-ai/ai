//! Combined catalogs include only enabled providers and keep provider-owned ordering.

use ai_interface::ModelSpec;

use crate::{Registry, catalog};

use super::support::unused_http;

#[test]
fn catalog_combines_exactly_enabled_provider_specs_in_stable_order() {
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
    let expected: Vec<ModelSpec> = catalogs
        .iter()
        .flat_map(|models| models.iter().cloned())
        .collect();

    assert_eq!(catalog().all(), expected);
    assert_eq!(Registry::new(unused_http()).catalog().all(), expected);
}
