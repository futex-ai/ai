//! Provider catalogs, authentication, and production adapter construction.

use std::collections::BTreeMap;
use std::iter;
use std::sync::Arc;

use ai_interface::{DynModel, Effort, ModelFeature, ModelSpec, ProviderId};
use ai_models_registry::{ProviderConfig, Registry, catalog};
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LiveProvider {
    Anthropic,
    DeepSeek,
    Google,
    Kimi,
    MiniMax,
    OpenAi,
    Qwen,
    Xai,
}

#[derive(Clone, Debug)]
/// One paid catalog probe with an explicit request effort.
pub(super) struct LiveProbe {
    /// Provider model facts used to build the adapter.
    pub(super) spec: ModelSpec,
    /// Existing catalog effort selected for this probe.
    pub(super) effort: Option<Effort>,
}

impl LiveProvider {
    pub(super) const ALL: [Self; 8] = [
        Self::Anthropic,
        Self::DeepSeek,
        Self::Google,
        Self::Kimi,
        Self::MiniMax,
        Self::OpenAi,
        Self::Qwen,
        Self::Xai,
    ];

    pub(super) fn from_id(provider: ProviderId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.provider_id() == provider)
    }

    pub(super) fn provider_id(self) -> ProviderId {
        match self {
            Self::Anthropic => ai_models_anthropic::PROVIDER_ID,
            Self::DeepSeek => ai_models_compat::deepseek::PROVIDER_ID,
            Self::Google => ai_models_google::PROVIDER_ID,
            Self::Kimi => ai_models_compat::kimi::PROVIDER_ID,
            Self::MiniMax => ai_models_compat::minimax::PROVIDER_ID,
            Self::OpenAi => ai_models_openai::PROVIDER_ID,
            Self::Qwen => ai_models_compat::qwen::PROVIDER_ID,
            Self::Xai => ai_models_compat::xai::PROVIDER_ID,
        }
    }

    pub(super) fn workflow_test(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic_catalog",
            Self::DeepSeek => "deepseek_catalog",
            Self::Google => "google_catalog",
            Self::Kimi => "kimi_catalog",
            Self::MiniMax => "minimax_catalog",
            Self::OpenAi => "openai_catalog",
            Self::Qwen => "qwen_catalog",
            Self::Xai => "xai_catalog",
        }
    }

    pub(super) fn workflow_secret(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::DeepSeek => "DEEPSEEK_API_KEY",
            Self::Google => "GOOGLE_API_KEY",
            Self::Kimi => "KIMI_API_KEY",
            Self::MiniMax => "MINIMAX_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Qwen => "QWEN_API_KEY",
            Self::Xai => "XAI_API_KEY",
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

    pub(super) fn chat_catalog(self) -> Vec<ModelSpec> {
        self.catalog()
            .into_iter()
            .filter(|model| {
                !model.has_feature(ModelFeature::ImageGeneration)
                    && !model.has_feature(ModelFeature::Judgment)
                    && !model.has_feature(ModelFeature::VideoGeneration)
            })
            .collect()
    }

    /// Expands each chat spec with its default effort first.
    pub(super) fn chat_probes(self) -> Vec<LiveProbe> {
        self.chat_catalog()
            .into_iter()
            .flat_map(|spec| {
                let default_effort = spec.default_effort;
                iter::once(default_effort)
                    .chain(
                        spec.efforts
                            .iter()
                            .map(Some)
                            .filter(move |effort| *effort != default_effort),
                    )
                    .map(move |effort| LiveProbe {
                        spec: spec.clone(),
                        effort,
                    })
            })
            .collect()
    }

    pub(super) fn auth(self, api_key: String) -> DynJsonHttpAuth {
        let auth = match self {
            Self::Anthropic => {
                StaticHeaderAuth::new(BTreeMap::from([("x-api-key".to_owned(), api_key)]))
            }
            Self::Google => {
                StaticHeaderAuth::new(BTreeMap::from([("x-goog-api-key".to_owned(), api_key)]))
            }
            _ => StaticHeaderAuth::bearer_token(api_key),
        };
        Arc::new(auth)
    }

    pub(super) fn build(
        self,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
        spec: &ModelSpec,
    ) -> DynModel {
        Registry::new(client)
            .with_provider(ProviderConfig::with_auth(self.provider_id(), auth))
            .model(spec)
            .expect("registered model provider and config")
    }
}

#[test]
fn live_probe_efforts_keep_the_default_first_without_duplicate_pairs() {
    let probes = LiveProvider::Kimi.chat_probes();

    assert_eq!(
        probes.iter().map(|probe| probe.effort).collect::<Vec<_>>(),
        vec![Some(Effort::Max), Some(Effort::Low), Some(Effort::High)]
    );
}
