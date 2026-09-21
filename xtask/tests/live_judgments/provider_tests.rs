//! Judgment-provider catalogs, authentication, and adapter construction tests.

use std::{collections::BTreeSet, sync::Arc};

use ai_interface::{DynJudgmentModel, ProviderKind};
use ai_models_core::{KnownModelSpec, ModelFeature};
use ai_models_typesafe::TypeSafeJudgmentModel;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveJudgmentProvider {
    TypeSafe,
}

impl LiveJudgmentProvider {
    pub(super) const ALL: [Self; 1] = [Self::TypeSafe];

    pub(super) fn from_kind(provider: ProviderKind) -> Option<Self> {
        match provider {
            ProviderKind::TypeSafe => Some(Self::TypeSafe),
            ProviderKind::Mock
            | ProviderKind::OpenAi
            | ProviderKind::Anthropic
            | ProviderKind::DeepSeek
            | ProviderKind::Google
            | ProviderKind::Kimi
            | ProviderKind::MiniMax
            | ProviderKind::Qwen
            | ProviderKind::Xai => None,
        }
    }

    pub(super) fn kind(self) -> ProviderKind {
        match self {
            Self::TypeSafe => ProviderKind::TypeSafe,
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

    pub(super) fn catalog(self) -> Vec<KnownModelSpec> {
        match self {
            Self::TypeSafe => ai_models_typesafe::known_models(),
        }
    }

    pub(super) fn judgment_catalog(self) -> Vec<KnownModelSpec> {
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
        spec: &KnownModelSpec,
    ) -> DynJudgmentModel {
        match self {
            Self::TypeSafe => Arc::new(TypeSafeJudgmentModel::with_auth(
                client,
                spec.provider_model_id,
                auth,
            )),
        }
    }
}

fn filter_judgment_catalog(catalog: Vec<KnownModelSpec>) -> Vec<KnownModelSpec> {
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
        .map(|provider| provider.kind())
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
            model.provider == provider.kind() && model.features == [ModelFeature::Judgment]
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
            LiveJudgmentProvider::from_kind(provider.kind()),
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
        assert_eq!(LiveJudgmentProvider::from_kind(provider), None);
    }
}

fn all_known_models() -> Vec<KnownModelSpec> {
    [
        ai_models_anthropic::known_models(),
        ai_models_deepseek::known_models(),
        ai_models_google::known_models(),
        ai_models_kimi::known_models(),
        ai_models_minimax::known_models(),
        ai_models_openai::known_models(),
        ai_models_qwen::known_models(),
        ai_models_typesafe::known_models(),
        ai_models_xai::known_models(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn non_judgment_provider_kinds() -> [ProviderKind; 9] {
    [
        ProviderKind::Mock,
        ProviderKind::OpenAi,
        ProviderKind::Anthropic,
        ProviderKind::DeepSeek,
        ProviderKind::Google,
        ProviderKind::Kimi,
        ProviderKind::MiniMax,
        ProviderKind::Qwen,
        ProviderKind::Xai,
    ]
}
