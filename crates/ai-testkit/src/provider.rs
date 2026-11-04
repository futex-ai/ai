//! Provider catalogs and adapter construction at the test composition boundary.

use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use ai_interface::{DynModel, Effort, ModelCatalog, ModelFeature, ModelSpec};
use ai_models_anthropic::AnthropicModel;
use ai_models_compat::deepseek::DeepSeekModel;
use ai_models_compat::kimi::KimiModel;
use ai_models_compat::minimax::MiniMaxModel;
use ai_models_compat::qwen::QwenModel;
use ai_models_compat::xai::XaiModel;
use ai_models_google::GoogleModel;
use ai_models_openai::OpenAiModel;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    scenario::Scenario,
};

/// One chat provider supported by the conformance runner.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic Messages.
    Anthropic,
    /// OpenAI Responses.
    Openai,
    /// Google Gemini.
    Google,
    /// DeepSeek Chat Completions.
    Deepseek,
    /// Kimi Chat Completions.
    Kimi,
    /// Qwen Chat Completions.
    Qwen,
    /// MiniMax Chat Completions.
    Minimax,
    /// xAI synchronous Chat Completions.
    Xai,
}

impl Provider {
    /// Stable provider traversal order.
    pub const ALL: [Self; 8] = [
        Self::Anthropic,
        Self::Openai,
        Self::Google,
        Self::Deepseek,
        Self::Kimi,
        Self::Qwen,
        Self::Minimax,
        Self::Xai,
    ];

    /// Directory name and CLI argument.
    pub fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Openai => "openai",
            Self::Google => "google",
            Self::Deepseek => "deepseek",
            Self::Kimi => "kimi",
            Self::Qwen => "qwen",
            Self::Minimax => "minimax",
            Self::Xai => "xai",
        }
    }

    /// Chat-only catalog. Image, video, and judgment adapters are excluded.
    pub fn catalog(self) -> Vec<ModelSpec> {
        let catalog = match self {
            Self::Anthropic => ai_models_anthropic::CATALOG.to_vec(),
            Self::Openai => ai_models_openai::CATALOG.to_vec(),
            Self::Google => ai_models_google::CATALOG.to_vec(),
            Self::Deepseek => ai_models_compat::deepseek::CATALOG.to_vec(),
            Self::Kimi => ai_models_compat::kimi::CATALOG.to_vec(),
            Self::Qwen => ai_models_compat::qwen::CATALOG.to_vec(),
            Self::Minimax => ai_models_compat::minimax::CATALOG.to_vec(),
            Self::Xai => ai_models_compat::xai::CATALOG.to_vec(),
        };
        catalog
            .into_iter()
            .filter(|model| {
                ![
                    ModelFeature::ImageGeneration,
                    ModelFeature::VideoGeneration,
                    ModelFeature::Judgment,
                ]
                .iter()
                .any(|feature| model.has_feature(*feature))
            })
            .collect()
    }

    /// Selects the first chat entry, preferring enabled thinking for a tool loop.
    pub fn model(self, scenario: Scenario) -> Result<ModelSpec> {
        let catalog = self.catalog();
        let model = if scenario == Scenario::ToolLoop {
            catalog
                .iter()
                .find(|model| model.default_effort.is_none_or(Effort::is_enabled))
                .or_else(|| catalog.first())
        } else {
            catalog.first()
        };
        match model {
            Some(model) => Ok(model.clone()),
            None => Err(Error::UnknownModel {
                id: self.name().to_owned(),
            }),
        }
    }

    /// Uses each provider's existing auth hook shape.
    pub fn auth(self, key: String) -> DynJsonHttpAuth {
        let auth = match self {
            Self::Anthropic => {
                StaticHeaderAuth::new(BTreeMap::from([("x-api-key".to_owned(), key)]))
            }
            Self::Google => {
                StaticHeaderAuth::new(BTreeMap::from([("x-goog-api-key".to_owned(), key)]))
            }
            _ => StaticHeaderAuth::bearer_token(key),
        };
        Arc::new(auth)
    }
}

impl FromStr for Provider {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|provider| provider.name() == name)
            .ok_or_else(|| Error::UnknownProvider {
                name: name.to_owned(),
            })
    }
}

/// Injectable adapter factory for replay and live recording.
pub trait ModelFactory: Send + Sync {
    /// Builds an adapter from a stored provider/model text id and an injected client.
    fn build(
        &self,
        provider: Provider,
        id: &str,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
    ) -> Result<DynModel>;
}

/// Builds today's production adapters without wrappers or retries.
pub struct ProviderModelFactory;

impl ModelFactory for ProviderModelFactory {
    fn build(
        &self,
        provider: Provider,
        id: &str,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
    ) -> Result<DynModel> {
        let catalog = ModelCatalog::new().with_models(provider.catalog());
        let Some(spec) = catalog.find(id).cloned() else {
            return Err(Error::UnknownModel { id: id.to_owned() });
        };
        let model: DynModel = match provider {
            Provider::Anthropic => Arc::new(AnthropicModel::with_catalog_auth(client, spec, auth)),
            Provider::Openai => Arc::new(OpenAiModel::with_catalog_auth(client, spec, auth)),
            Provider::Google => Arc::new(GoogleModel::with_catalog_auth(client, spec, auth)),
            Provider::Deepseek => Arc::new(DeepSeekModel::with_catalog_auth(client, spec, auth)),
            Provider::Kimi => Arc::new(KimiModel::with_catalog_auth(client, spec, auth)),
            Provider::Qwen => Arc::new(QwenModel::with_catalog_auth(client, spec, auth)),
            Provider::Minimax => Arc::new(MiniMaxModel::with_catalog_auth(client, spec, auth)),
            Provider::Xai => Arc::new(XaiModel::with_catalog_auth(client, spec, auth)),
        };
        Ok(model)
    }
}
