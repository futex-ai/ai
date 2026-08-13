//! Image-provider catalogs, authentication, and adapter construction tests.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use ai_interface::{DynImageGenerator, ProviderKind};
use ai_models_core::{KnownModelSpec, ModelFeature};
use ai_models_google::GoogleImageGenerator;
use ai_models_openai::OpenAiImageGenerator;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveImageProvider {
    Google,
    OpenAi,
}

impl LiveImageProvider {
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
            Self::Google => "google_image_catalog",
            Self::OpenAi => "openai_image_catalog",
        }
    }

    pub(super) fn workflow_secret(self) -> &'static str {
        match self {
            Self::Google => "GOOGLE_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
        }
    }

    pub(super) fn catalog(self) -> Vec<KnownModelSpec> {
        match self {
            Self::Google => ai_models_google::known_models(),
            Self::OpenAi => ai_models_openai::known_models(),
        }
    }

    pub(super) fn image_catalog(self) -> Vec<KnownModelSpec> {
        self.catalog()
            .into_iter()
            .filter(|model| model.has_feature(ModelFeature::ImageGeneration))
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
    ) -> DynImageGenerator {
        match self {
            Self::Google => Arc::new(GoogleImageGenerator::with_auth(
                client,
                spec.provider_model_id,
                auth,
            )),
            Self::OpenAi => Arc::new(OpenAiImageGenerator::with_auth(
                client,
                spec.provider_model_id,
                auth,
            )),
        }
    }
}

#[test]
fn registry_covers_every_image_capable_provider() {
    let expected = all_known_models()
        .into_iter()
        .filter(|model| model.has_feature(ModelFeature::ImageGeneration))
        .map(|model| model.provider)
        .collect::<BTreeSet<_>>();
    let actual = LiveImageProvider::ALL
        .iter()
        .map(|provider| provider.kind())
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn every_registered_provider_has_only_its_image_entries() {
    for provider in LiveImageProvider::ALL {
        let catalog = provider.catalog();
        let image_catalog = provider.image_catalog();

        assert!(
            !catalog.is_empty(),
            "{provider:?} catalog must not be empty"
        );
        assert!(
            !image_catalog.is_empty(),
            "{provider:?} image catalog must not be empty"
        );
        assert!(
            image_catalog.iter().all(|model| {
                model.provider == provider.kind()
                    && model.has_feature(ModelFeature::ImageGeneration)
            }),
            "{provider:?} image catalog contained an invalid entry"
        );
    }
}

#[test]
fn image_catalog_excludes_non_image_entries() {
    for provider in LiveImageProvider::ALL {
        assert!(
            provider.catalog().len() > provider.image_catalog().len(),
            "{provider:?} test requires a non-image exclusion case"
        );
        assert!(
            provider
                .image_catalog()
                .iter()
                .all(|model| model.has_feature(ModelFeature::ImageGeneration)),
            "{provider:?} image catalog included a non-image model"
        );
    }
}

#[test]
fn registry_constructs_every_dynamic_image_adapter_without_network_access() {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());

    for provider in LiveImageProvider::ALL {
        assert_eq!(
            LiveImageProvider::from_kind(provider.kind()),
            Some(provider)
        );
        let auth = provider.auth("credential-free-construction-key".to_owned());
        for model in provider.image_catalog() {
            let _generator = provider.build(client.clone(), auth.clone(), &model);
        }
    }

    for provider in non_image_provider_kinds() {
        assert_eq!(LiveImageProvider::from_kind(provider), None);
    }
}

fn all_known_models() -> Vec<ai_models_core::KnownModelSpec> {
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

fn non_image_provider_kinds() -> [ProviderKind; 7] {
    [
        ProviderKind::Mock,
        ProviderKind::Anthropic,
        ProviderKind::DeepSeek,
        ProviderKind::Kimi,
        ProviderKind::MiniMax,
        ProviderKind::Qwen,
        ProviderKind::Xai,
    ]
}
