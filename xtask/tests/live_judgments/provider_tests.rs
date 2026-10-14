//! Judgment-provider catalogs, authentication, and adapter construction tests.

use std::{collections::BTreeSet, sync::Arc};

use ai_interface::{DynJudgmentModel, ProviderKind};
use ai_models_core::{KnownModelSpec, ModelFeature};
use ai_models_openai::OpenAiJudgmentModel;
use ai_models_typesafe::TypeSafeJudgmentModel;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveJudgmentProvider {
    OpenAi,
    TypeSafe,
}

impl LiveJudgmentProvider {
    pub(super) const ALL: [Self; 2] = [Self::OpenAi, Self::TypeSafe];

    pub(super) fn from_kind(provider: ProviderKind) -> Option<Self> {
        match provider {
            ProviderKind::OpenAi => Some(Self::OpenAi),
            ProviderKind::TypeSafe => Some(Self::TypeSafe),
            ProviderKind::Mock
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
            Self::OpenAi => ProviderKind::OpenAi,
            Self::TypeSafe => ProviderKind::TypeSafe,
        }
    }

    pub(super) fn workflow_test(self) -> &'static str {
        match self {
            Self::OpenAi => "catalog_tests::openai_judgment_catalog",
            Self::TypeSafe => "catalog_tests::typesafe_judgment_catalog",
        }
    }

    pub(super) fn workflow_secret(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::TypeSafe => "TYPESAFE_API_KEY",
        }
    }

    pub(super) fn catalog(self) -> Vec<KnownModelSpec> {
        match self {
            Self::OpenAi => ai_models_openai::known_models(),
            Self::TypeSafe => ai_models_typesafe::known_models(),
        }
    }

    pub(super) fn judgment_catalog(self) -> Vec<KnownModelSpec> {
        filter_judgment_catalog(self.catalog())
    }

    pub(super) fn auth(self, api_key: String) -> DynJsonHttpAuth {
        match self {
            Self::OpenAi | Self::TypeSafe => Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        }
    }

    pub(super) fn build(
        self,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
        spec: &KnownModelSpec,
    ) -> DynJudgmentModel {
        match self {
            Self::OpenAi => Arc::new(OpenAiJudgmentModel::with_auth(
                client,
                spec.provider_model_id,
                auth,
            )),
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
fn every_registered_provider_has_judgment_only_entries() {
    for provider in LiveJudgmentProvider::ALL {
        let catalog = provider.catalog();
        let judgment_catalog = provider.judgment_catalog();

        assert!(
            !catalog.is_empty(),
            "{provider:?} catalog must not be empty"
        );
        assert!(
            !judgment_catalog.is_empty(),
            "{provider:?} judgment catalog must not be empty"
        );
        assert!(
            judgment_catalog.iter().all(|model| {
                model.provider == provider.kind() && model.features == [ModelFeature::Judgment]
            }),
            "{provider:?} judgment catalog contained an entry that is not judgment-only"
        );
    }
}

#[test]
fn typesafe_catalog_is_entirely_judgment_models() {
    let provider = LiveJudgmentProvider::TypeSafe;

    assert_eq!(provider.judgment_catalog(), provider.catalog());
}

#[test]
fn openai_catalog_keeps_its_chat_entries_outside_the_judgment_catalog() {
    let provider = LiveJudgmentProvider::OpenAi;
    let catalog = provider.catalog();
    let judgment_catalog = provider.judgment_catalog();

    assert!(catalog.len() > judgment_catalog.len());
    assert!(
        judgment_catalog
            .iter()
            .all(|model| model.provider_model_id == ai_models_openai::GPT_6_LUNA)
    );
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

fn non_judgment_provider_kinds() -> [ProviderKind; 8] {
    [
        ProviderKind::Mock,
        ProviderKind::Anthropic,
        ProviderKind::DeepSeek,
        ProviderKind::Google,
        ProviderKind::Kimi,
        ProviderKind::MiniMax,
        ProviderKind::Qwen,
        ProviderKind::Xai,
    ]
}
