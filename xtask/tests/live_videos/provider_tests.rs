//! Video-provider catalogs, authentication, and adapter construction tests.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use ai_interface::{DynVideoGenerator, ModelFeature, ModelSpec, ProviderId};
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

    pub(super) fn catalog(self) -> Vec<ModelSpec> {
        let catalog = match self {
            Self::Google => ai_models_google::CATALOG.to_vec(),
            Self::OpenAi => ai_models_openai::CATALOG.to_vec(),
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
        spec: &ModelSpec,
    ) -> DynVideoGenerator {
        match self {
            Self::Google => Arc::new(GoogleVideoGenerator::with_auth(
                client,
                spec.provider_model_id.as_ref(),
                auth,
            )),
            Self::OpenAi => Arc::new(OpenAiVideoGenerator::with_auth(
                client,
                spec.provider_model_id.as_ref(),
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
        .map(|provider| provider.provider_id())
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn registered_catalogs_are_non_empty_and_constructible() {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    for provider in LiveVideoProvider::ALL {
        assert_eq!(
            LiveVideoProvider::from_id(provider.provider_id()),
            Some(provider)
        );
        let catalog = provider.catalog();
        assert!(!catalog.is_empty(), "{provider:?} video catalog is empty");
        assert!(catalog.iter().all(|model| {
            model.provider == provider.provider_id()
                && model.has_feature(ModelFeature::VideoGeneration)
                && !model.has_feature(ModelFeature::ImageGeneration)
        }));
        let auth = provider.auth("credential-free-key".to_owned());
        for model in catalog {
            let _generator = provider.build(client.clone(), auth.clone(), &model);
        }
    }

    for provider in [
        ai_interface::MOCK_PROVIDER,
        ai_models_anthropic::PROVIDER_ID,
        ai_models_compat::deepseek::PROVIDER_ID,
        ai_models_compat::kimi::PROVIDER_ID,
        ai_models_compat::minimax::PROVIDER_ID,
        ai_models_compat::qwen::PROVIDER_ID,
        ai_models_typesafe::PROVIDER_ID,
        ai_models_compat::xai::PROVIDER_ID,
    ] {
        assert_eq!(LiveVideoProvider::from_id(provider), None);
    }
}

fn all_known_models() -> Vec<ModelSpec> {
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
