//! Image-provider catalogs, authentication, and adapter construction tests.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use ai_interface::{DynImageGenerator, ModelFeature, ModelSpec, ProviderId};
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

    pub(super) fn from_id(provider: ProviderId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.provider_id() == provider)
    }

    pub(super) fn provider_id(self) -> ProviderId {
        match self {
            Self::Google => ai_models_google::PROVIDER_ID,
            Self::OpenAi => ai_models_openai::PROVIDER_ID,
        }
    }

    pub(super) fn workflow_test(self) -> &'static str {
        match self {
            Self::Google => "catalog_tests::google_image_catalog",
            Self::OpenAi => "catalog_tests::openai_image_catalog",
        }
    }

    pub(super) fn workflow_secret(self) -> &'static str {
        match self {
            Self::Google => "GOOGLE_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
        }
    }

    pub(super) fn catalog(self) -> Vec<ModelSpec> {
        match self {
            Self::Google => ai_models_google::CATALOG.to_vec(),
            Self::OpenAi => ai_models_openai::CATALOG.to_vec(),
        }
    }

    pub(super) fn image_catalog(self) -> Vec<ModelSpec> {
        filter_image_catalog(self.catalog())
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
        spec: &ModelSpec,
    ) -> DynImageGenerator {
        match self {
            Self::Google => Arc::new(GoogleImageGenerator::with_auth(
                client,
                spec.provider_model_id.as_ref(),
                auth,
            )),
            Self::OpenAi => Arc::new(OpenAiImageGenerator::with_auth(
                client,
                spec.provider_model_id.as_ref(),
                auth,
            )),
        }
    }
}

fn filter_image_catalog(catalog: Vec<ModelSpec>) -> Vec<ModelSpec> {
    catalog
        .into_iter()
        .filter(|model| model.has_feature(ModelFeature::ImageGeneration))
        .collect()
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
        .map(|provider| provider.provider_id())
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
                model.provider == provider.provider_id()
                    && model.has_feature(ModelFeature::ImageGeneration)
            }),
            "{provider:?} image catalog contained an invalid entry"
        );
    }
}

#[test]
fn image_only_catalog_is_a_valid_filter_input() {
    let catalog = LiveImageProvider::ALL
        .into_iter()
        .flat_map(LiveImageProvider::catalog)
        .filter(|model| model.has_feature(ModelFeature::ImageGeneration))
        .collect::<Vec<_>>();

    assert!(!catalog.is_empty(), "expected image-only catalog entries");
    assert_eq!(filter_image_catalog(catalog.clone()), catalog);
}

#[test]
fn image_catalog_excludes_non_image_entries() {
    let catalog = LiveImageProvider::ALL
        .into_iter()
        .flat_map(LiveImageProvider::catalog)
        .collect::<Vec<_>>();
    let image_catalog = filter_image_catalog(catalog.clone());

    assert!(
        catalog.len() > image_catalog.len(),
        "registered catalogs must provide a global non-image exclusion case"
    );
    assert!(
        image_catalog
            .iter()
            .all(|model| model.has_feature(ModelFeature::ImageGeneration)),
        "image catalog included a non-image model"
    );
}

#[test]
fn registry_constructs_every_dynamic_image_adapter_without_network_access() {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());

    for provider in LiveImageProvider::ALL {
        assert_eq!(
            LiveImageProvider::from_id(provider.provider_id()),
            Some(provider)
        );
        let auth = provider.auth("credential-free-construction-key".to_owned());
        for model in provider.image_catalog() {
            let _generator = provider.build(client.clone(), auth.clone(), &model);
        }
    }

    for provider in non_image_provider_kinds() {
        assert_eq!(LiveImageProvider::from_id(provider), None);
    }
}

fn all_known_models() -> Vec<ai_interface::ModelSpec> {
    [
        ai_models_anthropic::CATALOG.to_vec(),
        ai_models_compat::deepseek::CATALOG.to_vec(),
        ai_models_google::CATALOG.to_vec(),
        ai_models_compat::kimi::CATALOG.to_vec(),
        ai_models_compat::minimax::CATALOG.to_vec(),
        ai_models_openai::CATALOG.to_vec(),
        ai_models_compat::qwen::CATALOG.to_vec(),
        ai_models_compat::xai::CATALOG.to_vec(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn non_image_provider_kinds() -> [ProviderId; 8] {
    [
        ai_interface::MOCK_PROVIDER,
        ai_models_anthropic::PROVIDER_ID,
        ai_models_compat::deepseek::PROVIDER_ID,
        ai_models_compat::kimi::PROVIDER_ID,
        ai_models_compat::minimax::PROVIDER_ID,
        ai_models_compat::qwen::PROVIDER_ID,
        ai_models_typesafe::PROVIDER_ID,
        ai_models_compat::xai::PROVIDER_ID,
    ]
}
