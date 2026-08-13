//! Video-provider catalogs, authentication, and adapter construction tests.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use ai_interface::{DynVideoGenerator, ProviderKind};
use ai_models_core::{KnownModelSpec, ModelFeature};
use ai_models_google::GoogleVideoGenerator;
use ai_models_openai::OpenAiVideoGenerator;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveVideoProvider {
    Google,
    OpenAi,
}

impl LiveVideoProvider {
    pub(super) const ALL: [Self; 2] = [Self::Google, Self::OpenAi];

    pub(super) fn from_kind(provider: ProviderKind) -> Option<Self> {
        match provider {
            ProviderKind::Google => Some(Self::Google),
            ProviderKind::OpenAi => Some(Self::OpenAi),
            ProviderKind::Mock
            | ProviderKind::Anthropic
            | ProviderKind::DeepSeek
            | ProviderKind::Kimi
            | ProviderKind::MiniMax
            | ProviderKind::Qwen
            | ProviderKind::Xai => None,
        }
    }

    pub(super) fn kind(self) -> ProviderKind {
        match self {
            Self::Google => ProviderKind::Google,
            Self::OpenAi => ProviderKind::OpenAi,
        }
    }

    pub(super) fn workflow_test(self) -> &'static str {
        match self {
            Self::Google => "catalog_tests::google_video_catalog",
            Self::OpenAi => "catalog_tests::openai_video_catalog",
        }
    }

    pub(super) fn workflow_secret(self) -> &'static str {
        match self {
            Self::Google => "GOOGLE_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
        }
    }

    pub(super) fn catalog(self) -> Vec<KnownModelSpec> {
        let catalog = match self {
            Self::Google => ai_models_google::known_models(),
            Self::OpenAi => ai_models_openai::known_models(),
        };
        catalog
            .into_iter()
            .filter(|model| model.has_feature(ModelFeature::VideoGeneration))
            .collect()
    }

    pub(super) fn auth(self, api_key: String) -> DynJsonHttpAuth {
        let auth = match self {
            Self::Google => {
                StaticHeaderAuth::new(BTreeMap::from([("x-goog-api-key".to_owned(), api_key)]))
            }
            Self::OpenAi => StaticHeaderAuth::bearer_token(api_key),
        };
        Arc::new(auth)
    }

    pub(super) fn build(
        self,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
        spec: &KnownModelSpec,
    ) -> DynVideoGenerator {
        match self {
            Self::Google => Arc::new(GoogleVideoGenerator::with_auth(
                client,
                spec.provider_model_id,
                auth,
            )),
            Self::OpenAi => Arc::new(OpenAiVideoGenerator::with_auth(
                client,
                spec.provider_model_id,
                auth,
            )),
        }
    }
}

#[test]
fn registry_covers_every_video_capable_provider() {
    let expected = all_known_models()
        .into_iter()
        .filter(|model| model.has_feature(ModelFeature::VideoGeneration))
        .map(|model| model.provider)
        .collect::<BTreeSet<_>>();
    let actual = LiveVideoProvider::ALL
        .iter()
        .map(|provider| provider.kind())
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn registered_catalogs_are_non_empty_and_constructible() {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    for provider in LiveVideoProvider::ALL {
        assert_eq!(
            LiveVideoProvider::from_kind(provider.kind()),
            Some(provider)
        );
        let catalog = provider.catalog();
        assert!(!catalog.is_empty(), "{provider:?} video catalog is empty");
        assert!(catalog.iter().all(|model| {
            model.provider == provider.kind()
                && model.has_feature(ModelFeature::VideoGeneration)
                && !model.has_feature(ModelFeature::ImageGeneration)
        }));
        let auth = provider.auth("credential-free-key".to_owned());
        for model in catalog {
            let _generator = provider.build(client.clone(), auth.clone(), &model);
        }
    }

    for provider in [
        ProviderKind::Mock,
        ProviderKind::Anthropic,
        ProviderKind::DeepSeek,
        ProviderKind::Kimi,
        ProviderKind::MiniMax,
        ProviderKind::Qwen,
        ProviderKind::Xai,
    ] {
        assert_eq!(LiveVideoProvider::from_kind(provider), None);
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
        ai_models_xai::known_models(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
