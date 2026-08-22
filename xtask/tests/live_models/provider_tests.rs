//! Provider catalogs, authentication, and production adapter construction.

use std::collections::BTreeMap;
use std::sync::Arc;

use ai_interface::{DynModel, ProviderKind};
use ai_models_anthropic::AnthropicModel;
use ai_models_core::{KnownModelSpec, ModelFeature};
use ai_models_deepseek::DeepSeekModel;
use ai_models_google::GoogleModel;
use ai_models_kimi::KimiModel;
use ai_models_minimax::MiniMaxModel;
use ai_models_openai::OpenAiModel;
use ai_models_qwen::QwenModel;
use ai_models_xai::XaiModel;
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
    Silent,
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

    pub(super) fn from_kind(provider: ProviderKind) -> Option<Self> {
        match provider {
            ProviderKind::Mock => None,
            ProviderKind::Anthropic => Some(Self::Anthropic),
            ProviderKind::DeepSeek => Some(Self::DeepSeek),
            ProviderKind::Google => Some(Self::Google),
            ProviderKind::Kimi => Some(Self::Kimi),
            ProviderKind::MiniMax => Some(Self::MiniMax),
            ProviderKind::OpenAi => Some(Self::OpenAi),
            ProviderKind::Qwen => Some(Self::Qwen),
            ProviderKind::Xai => Some(Self::Xai),
        }
    }

    pub(super) fn kind(self) -> ProviderKind {
        match self {
            Self::Anthropic => ProviderKind::Anthropic,
            Self::DeepSeek => ProviderKind::DeepSeek,
            Self::Google => ProviderKind::Google,
            Self::Kimi => ProviderKind::Kimi,
            Self::MiniMax => ProviderKind::MiniMax,
            Self::OpenAi => ProviderKind::OpenAi,
            Self::Qwen => ProviderKind::Qwen,
            Self::Xai => ProviderKind::Xai,
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
        match self {
            Self::Xai => CompletionEventExpectation::Silent,
            Self::Anthropic
            | Self::DeepSeek
            | Self::Google
            | Self::Kimi
            | Self::MiniMax
            | Self::OpenAi
            | Self::Qwen => CompletionEventExpectation::AssistantTextParity,
        }
    }

    pub(super) fn catalog(self) -> Vec<KnownModelSpec> {
        match self {
            Self::Anthropic => ai_models_anthropic::known_models(),
            Self::DeepSeek => ai_models_deepseek::known_models(),
            Self::Google => ai_models_google::known_models(),
            Self::Kimi => ai_models_kimi::known_models(),
            Self::MiniMax => ai_models_minimax::known_models(),
            Self::OpenAi => ai_models_openai::known_models(),
            Self::Qwen => ai_models_qwen::known_models(),
            Self::Xai => ai_models_xai::known_models(),
        }
    }

    pub(super) fn chat_catalog(self) -> Vec<KnownModelSpec> {
        self.catalog()
            .into_iter()
            .filter(|model| {
                !model.has_feature(ModelFeature::ImageGeneration)
                    && !model.has_feature(ModelFeature::VideoGeneration)
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
        spec: &KnownModelSpec,
    ) -> DynModel {
        match self {
            Self::Anthropic => Arc::new(AnthropicModel::with_catalog_auth(
                client,
                spec.id,
                spec.provider_model_id,
                spec.thinking_level,
                auth,
            )),
            Self::DeepSeek => Arc::new(
                DeepSeekModel::with_catalog_auth(
                    client,
                    spec.id,
                    spec.provider_model_id,
                    spec.thinking_level,
                    auth,
                )
                .expect("DeepSeek catalog entry should be constructible"),
            ),
            Self::Google => Arc::new(GoogleModel::with_catalog_auth(
                client,
                spec.id,
                spec.provider_model_id,
                spec.thinking_level,
                auth,
            )),
            Self::Kimi => Arc::new(
                KimiModel::with_catalog_auth(
                    client,
                    spec.id,
                    spec.provider_model_id,
                    spec.thinking_level,
                    auth,
                )
                .expect("Kimi catalog entry should be constructible"),
            ),
            Self::MiniMax => Arc::new(MiniMaxModel::with_catalog_auth(
                client,
                spec.id,
                spec.provider_model_id,
                spec.thinking_level,
                auth,
            )),
            Self::OpenAi => Arc::new(OpenAiModel::with_catalog_auth(
                client,
                spec.id,
                spec.provider_model_id,
                spec.thinking_level,
                auth,
            )),
            Self::Qwen => Arc::new(
                QwenModel::with_catalog_auth(
                    client,
                    spec.id,
                    spec.provider_model_id,
                    spec.thinking_level,
                    auth,
                )
                .expect("Qwen catalog entry should be constructible"),
            ),
            Self::Xai => Arc::new(XaiModel::with_catalog_auth(
                client,
                spec.id,
                spec.provider_model_id,
                spec.thinking_level,
                auth,
            )),
        }
    }
}
