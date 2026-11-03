//! Judgment-provider catalogs, authentication, and adapter construction tests.

use std::{collections::BTreeSet, sync::Arc};

use ai_interface::{DynJudgmentModel, ModelFeature, ModelSpec, ProviderId};
use ai_models_registry::{ProviderConfig, Registry, catalog};
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveJudgmentProvider {
    TypeSafe,
}

impl LiveJudgmentProvider {
    pub(super) const ALL: [Self; 1] = [Self::TypeSafe];

    pub(super) fn from_id(provider: ProviderId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.provider_id() == provider)
    }

    pub(super) fn provider_id(self) -> ProviderId {
        match self {
            Self::TypeSafe => ai_models_typesafe::PROVIDER_ID,
        }
    }

    pub(super) fn workflow_test(self) -> &'static str {
        match self {
            Self::TypeSafe => "catalog_tests::typesafe_judgment_catalog",
        }
    }

    pub(super) fn workflow_secret(self) -> &'static str {
        match self {
            Self::TypeSafe => "TYPESAFE_API_KEY",
        }
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
            Self::TypeSafe => Arc::new(StaticHeaderAuth::bearer_token(api_key)),
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
fn every_registered_provider_has_only_judgment_entries() {
    for provider in LiveJudgmentProvider::ALL {
        let catalog = provider.catalog();
        let judgment_catalog = provider.judgment_catalog();

        assert!(
            !catalog.is_empty(),
            "{provider:?} catalog must not be empty"
        );
        assert_eq!(judgment_catalog, catalog);
        assert!(catalog.iter().all(|model| {
            model.provider == provider.provider_id()
                && model.features.as_ref() == [ModelFeature::Judgment]
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

fn non_judgment_provider_kinds() -> [ProviderId; 9] {
    [
        ai_interface::MOCK_PROVIDER,
        ai_models_openai::PROVIDER_ID,
        ai_models_anthropic::PROVIDER_ID,
        ai_models_compat::deepseek::PROVIDER_ID,
        ai_models_google::PROVIDER_ID,
        ai_models_compat::kimi::PROVIDER_ID,
        ai_models_compat::minimax::PROVIDER_ID,
        ai_models_compat::qwen::PROVIDER_ID,
        ai_models_compat::xai::PROVIDER_ID,
    ]
}
