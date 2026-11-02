//! Provider catalogs, authentication, and production adapter construction.

use std::collections::BTreeMap;
use std::iter;
use std::sync::Arc;

use ai_interface::{DynModel, Effort, ModelFeature, ModelSpec, ProviderId};
use ai_models_anthropic::AnthropicModel;
use ai_models_compat::deepseek::DeepSeekModel;
use ai_models_compat::kimi::KimiModel;
use ai_models_compat::minimax::MiniMaxModel;
use ai_models_compat::qwen::QwenModel;
use ai_models_compat::xai::XaiModel;
use ai_models_google::GoogleModel;
use ai_models_openai::OpenAiModel;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CompletionEventExpectation {
    AssistantTextParity,
}

#[derive(Clone, Debug)]
/// One paid catalog probe with an explicit request effort.
pub(super) struct LiveProbe {
    /// Provider model facts used to build the adapter.
    pub(super) spec: ModelSpec,
    /// Existing catalog effort selected for this probe.
    pub(super) effort: Effort,
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

    pub(super) fn synchronous_event_expectation(self) -> CompletionEventExpectation {
        match self {
            Self::Anthropic
            | Self::DeepSeek
            | Self::Google
            | Self::Kimi
            | Self::MiniMax
            | Self::OpenAi
            | Self::Qwen
            | Self::Xai => CompletionEventExpectation::AssistantTextParity,
        }
    }

    pub(super) fn preferred_mode_event_expectation(self) -> CompletionEventExpectation {
        self.synchronous_event_expectation()
    }

    pub(super) fn catalog(self) -> Vec<ModelSpec> {
        match self {
            Self::Anthropic => ai_models_anthropic::CATALOG.to_vec(),
            Self::DeepSeek => ai_models_compat::deepseek::CATALOG.to_vec(),
            Self::Google => ai_models_google::CATALOG.to_vec(),
            Self::Kimi => ai_models_compat::kimi::CATALOG.to_vec(),
            Self::MiniMax => ai_models_compat::minimax::CATALOG.to_vec(),
            Self::OpenAi => ai_models_openai::CATALOG.to_vec(),
            Self::Qwen => ai_models_compat::qwen::CATALOG.to_vec(),
            Self::Xai => ai_models_compat::xai::CATALOG.to_vec(),
        }
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
        match self {
            Self::Anthropic => Arc::new(AnthropicModel::with_catalog_auth(
                client,
                spec.clone(),
                auth,
            )),
            Self::DeepSeek => {
                Arc::new(DeepSeekModel::with_catalog_auth(client, spec.clone(), auth))
            }
            Self::Google => Arc::new(GoogleModel::with_catalog_auth(client, spec.clone(), auth)),
            Self::Kimi => Arc::new(KimiModel::with_catalog_auth(client, spec.clone(), auth)),
            Self::MiniMax => Arc::new(MiniMaxModel::with_catalog_auth(client, spec.clone(), auth)),
            Self::OpenAi => Arc::new(OpenAiModel::with_catalog_auth(client, spec.clone(), auth)),
            Self::Qwen => Arc::new(QwenModel::with_catalog_auth(client, spec.clone(), auth)),
            Self::Xai => Arc::new(XaiModel::with_catalog_auth(client, spec.clone(), auth)),
        }
    }
}

#[test]
fn live_probe_efforts_keep_the_default_first_without_duplicate_pairs() {
    let probes = LiveProvider::Kimi.chat_probes();

    assert_eq!(
        probes.iter().map(|probe| probe.effort).collect::<Vec<_>>(),
        vec![Effort::Max, Effort::Low, Effort::High]
    );
}
