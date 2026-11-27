//! Judgment-provider catalogs, authentication, and adapter construction tests.

use std::{collections::BTreeSet, sync::Arc};

use ai_interface::{DynJudgmentModel, ModelFeature, ModelSpec, ProviderId};
use ai_models_registry::{ProviderConfig, Registry, catalog};
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth,
};
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub(super) enum ApiKeyError {
    #[error("[xtask/live_judgments] {name} is missing")]
    Missing { name: String },
    #[error("[xtask/live_judgments] {name} is blank")]
    Blank { name: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveJudgmentProvider {
    TypeSafe,
    OpenAi,
}

impl LiveJudgmentProvider {
    pub(super) const ALL: [Self; 2] = [Self::TypeSafe, Self::OpenAi];

    pub(super) fn from_id(provider: ProviderId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.provider_id() == provider)
    }

    pub(super) fn provider_id(self) -> ProviderId {
        match self {
            Self::TypeSafe => ai_models_typesafe::PROVIDER_ID,
            Self::OpenAi => ai_models_openai::PROVIDER_ID,
        }
    }

    pub(super) fn workflow_test(self) -> &'static str {
        macro_rules! test_name {
            ($module:ident :: $test:ident) => {{
                let _: fn() = super::$module::$test;
                concat!(stringify!($module), "::", stringify!($test))
            }};
        }
        match self {
            Self::TypeSafe => test_name!(catalog_tests::typesafe_judgment_catalog),
            Self::OpenAi => test_name!(catalog_tests::openai_judgment_catalog),
        }
    }

    pub(super) fn workflow_secret(self) -> String {
        format!(
            "{}_API_KEY",
            self.provider_id().as_str().to_ascii_uppercase()
        )
    }

    pub(super) fn api_key_variable(self) -> String {
        format!("LIVE_JUDGMENT_{}", self.workflow_secret())
    }

    pub(super) fn require_api_key(
        self,
        read: impl FnOnce(&str) -> Option<String>,
    ) -> Result<String, ApiKeyError> {
        let name = self.api_key_variable();
        let Some(value) = read(&name) else {
            return Err(ApiKeyError::Missing { name });
        };
        if value.trim().is_empty() {
            return Err(ApiKeyError::Blank { name });
        }
        Ok(value)
    }

    pub(super) fn catalog(self) -> Vec<ModelSpec> {
        catalog()
            .all()
            .iter()
            .filter(|model| model.provider == self.provider_id())
            .cloned()
            .collect()
    }

    pub(super) fn judgment_catalog(self) -> Vec<ModelSpec> {
        filter_judgment_catalog(self.catalog())
    }

    pub(super) fn auth(self, api_key: String) -> DynJsonHttpAuth {
        match self {
            Self::TypeSafe | Self::OpenAi => Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        }
    }

    pub(super) fn build(
        self,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
        spec: &ModelSpec,
    ) -> DynJudgmentModel {
        Registry::new(client)
            .with_provider(ProviderConfig::with_auth(self.provider_id(), auth))
            .judgment(spec)
            .expect("registered judgment provider and config")
    }
}

fn filter_judgment_catalog(catalog: Vec<ModelSpec>) -> Vec<ModelSpec> {
    catalog
        .into_iter()
        .filter(|model| model.has_feature(ModelFeature::Judgment))
        .collect()
}

#[test]
fn requires_a_present_non_empty_api_key() {
    let provider = LiveJudgmentProvider::TypeSafe;
    assert!(matches!(
        provider.require_api_key(|_| None),
        Err(ApiKeyError::Missing { .. })
    ));
    assert!(matches!(
        provider.require_api_key(|_| Some(" \n\t".to_owned())),
        Err(ApiKeyError::Blank { .. })
    ));
    assert_eq!(
        provider
            .require_api_key(|name| {
                (name == "LIVE_JUDGMENT_TYPESAFE_API_KEY").then(|| " fixture-key ".to_owned())
            })
            .unwrap(),
        " fixture-key "
    );
}

#[test]
fn another_providers_key_does_not_unlock_the_catalog() {
    const OPENAI_KEY: &str = "LIVE_JUDGMENT_OPENAI_API_KEY";
    const TYPESAFE_KEY: &str = "LIVE_JUDGMENT_TYPESAFE_API_KEY";
    for (provider, other_key, own_key) in [
        (LiveJudgmentProvider::OpenAi, TYPESAFE_KEY, OPENAI_KEY),
        (LiveJudgmentProvider::TypeSafe, OPENAI_KEY, TYPESAFE_KEY),
    ] {
        let error = provider
            .require_api_key(|name| (name == other_key).then(|| "fixture-key".to_owned()))
            .unwrap_err();

        assert_eq!(
            error,
            ApiKeyError::Missing {
                name: own_key.to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            format!("[xtask/live_judgments] {own_key} is missing")
        );
    }
}

#[test]
fn registry_covers_every_judgment_capable_provider() {
    let expected = all_known_models()
        .into_iter()
        .filter(|model| model.has_feature(ModelFeature::Judgment))
        .map(|model| model.provider)
        .collect::<BTreeSet<_>>();
    let actual = LiveJudgmentProvider::ALL
        .iter()
        .map(|provider| provider.provider_id())
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn every_registered_provider_has_judgment_entries() {
    for provider in LiveJudgmentProvider::ALL {
        let catalog = provider.judgment_catalog();

        assert!(
            !catalog.is_empty(),
            "{provider:?} judgment catalog must not be empty"
        );
        assert!(catalog.iter().all(|model| {
            model.provider == provider.provider_id() && model.has_feature(ModelFeature::Judgment)
        }));
    }
}

#[test]
fn judgment_filter_excludes_non_judgment_entries() {
    let catalog = all_known_models();
    let judgment_catalog = filter_judgment_catalog(catalog.clone());

    assert!(!judgment_catalog.is_empty());
    assert!(catalog.len() > judgment_catalog.len());
    assert!(
        judgment_catalog
            .iter()
            .all(|model| model.has_feature(ModelFeature::Judgment))
    );
}

#[test]
fn openai_live_catalog_selects_only_its_judgment_specs() {
    let model_ids = LiveJudgmentProvider::from_id(ai_models_openai::PROVIDER_ID).map(|provider| {
        provider
            .judgment_catalog()
            .into_iter()
            .map(|model| model.text_id())
            .collect::<Vec<_>>()
    });

    assert_eq!(model_ids, Some(vec!["openai/gpt-6-luna".to_owned()]));
}

#[test]
fn registry_constructs_every_dynamic_judgment_adapter_without_network_access() {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());

    for provider in LiveJudgmentProvider::ALL {
        assert_eq!(
            LiveJudgmentProvider::from_id(provider.provider_id()),
            Some(provider)
        );
        let auth = provider.auth("credential-free-construction-key".to_owned());
        for model in provider.judgment_catalog() {
            let _model = provider.build(client.clone(), auth.clone(), &model);
        }
    }
}

#[test]
fn every_non_judgment_provider_kind_maps_to_none() {
    for provider in non_judgment_provider_kinds() {
        assert_eq!(LiveJudgmentProvider::from_id(provider), None);
    }
}

fn all_known_models() -> Vec<ModelSpec> {
    catalog().all().to_vec()
}

fn non_judgment_provider_kinds() -> [ProviderId; 8] {
    [
        ai_interface::MOCK_PROVIDER,
        ai_models_anthropic::PROVIDER_ID,
        ai_models_compat::deepseek::PROVIDER_ID,
        ai_models_google::PROVIDER_ID,
        ai_models_compat::kimi::PROVIDER_ID,
        ai_models_compat::minimax::PROVIDER_ID,
        ai_models_compat::qwen::PROVIDER_ID,
        ai_models_compat::xai::PROVIDER_ID,
    ]
}
