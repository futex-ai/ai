//! DeepSeek public construction and configuration errors.

use std::sync::Arc;

use ai_interface::{
    Model, ModelCompletionEventSink, ModelRequest, ModelResponse, ModelResult, ProviderKind,
};
use ai_models_core::{ThinkingLevel, resolve_catalog_thinking_level};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use thiserror::Error;

#[cfg(test)]
use ai_interface::ModelError;

use super::{DEEPSEEK_V4_PRO, catalog::known_models, dialect::DIALECT};
use crate::engine::CompatModel;

#[cfg(test)]
const PROVIDER: &str = "deepseek";

#[derive(Debug, Eq, Error, PartialEq)]
/// Invalid configuration rejected before a DeepSeek request can be sent.
pub enum DeepSeekConfigurationError {
    /// The adapter supports only current DeepSeek V4 Pro and Flash provider ids.
    #[error("[ai_models_deepseek/client] unsupported provider model `{provider_model_id}`")]
    UnsupportedProviderModel {
        /// Unsupported upstream model identifier.
        provider_model_id: String,
    },
    /// No supported DeepSeek thinking level exists at or below the request.
    #[error(
        "[ai_models_deepseek/client] no DeepSeek V4 thinking level at or below `{thinking_level}`"
    )]
    UnsupportedThinkingLevel {
        /// Unsupported normalized thinking-level value.
        thinking_level: &'static str,
    },
}

/// Result alias for DeepSeek model construction.
pub type DeepSeekConfigurationResult<T> = std::result::Result<T, DeepSeekConfigurationError>;

#[derive(Clone)]
/// DeepSeek-backed `ai_interface::Model` implementation.
pub struct DeepSeekModel {
    inner: CompatModel,
}

impl DeepSeekModel {
    /// Builds the default high-thinking DeepSeek V4 Pro model from an API key.
    pub fn new(http_client: DynJsonHttpClient, api_key: impl Into<String>) -> Self {
        Self::with_auth(
            http_client,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds the default high-thinking DeepSeek V4 Pro model from an auth hook.
    pub fn with_auth(http_client: DynJsonHttpClient, auth: DynJsonHttpAuth) -> Self {
        Self {
            inner: CompatModel::new(
                &DIALECT,
                http_client,
                DEEPSEEK_V4_PRO.to_owned(),
                DEEPSEEK_V4_PRO.to_owned(),
                ThinkingLevel::High,
                auth,
            ),
        }
    }

    /// Builds a DeepSeek model from validated catalog metadata and explicit auth.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> DeepSeekConfigurationResult<Self> {
        let provider_model_id = provider_model_id.into();
        let thinking_level = validate_configuration(&provider_model_id, thinking_level)?;
        Ok(Self {
            inner: CompatModel::new(
                &DIALECT,
                http_client,
                catalog_model_id.into(),
                provider_model_id,
                thinking_level,
                auth,
            ),
        })
    }
}

#[async_trait]
impl Model for DeepSeekModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.inner.complete(request).await
    }
    async fn complete_with_events(
        &self,
        request: &ModelRequest,
        sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        self.inner.complete_with_events(request, sink).await
    }
}

fn validate_configuration(
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
) -> DeepSeekConfigurationResult<ThinkingLevel> {
    if !matches!(
        provider_model_id,
        super::DEEPSEEK_V4_PRO | super::DEEPSEEK_V4_FLASH
    ) {
        return Err(DeepSeekConfigurationError::UnsupportedProviderModel {
            provider_model_id: provider_model_id.to_owned(),
        });
    }
    let models = known_models();
    match resolve_catalog_thinking_level(
        &models,
        ProviderKind::DeepSeek,
        provider_model_id,
        thinking_level,
    ) {
        Some(effective) => Ok(effective),
        None => Err(DeepSeekConfigurationError::UnsupportedThinkingLevel {
            thinking_level: thinking_level.as_str(),
        }),
    }
}

#[cfg(test)]
pub(super) fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { message } | json_http::Error::Auth { message } => {
            ModelError::transient_provider(PROVIDER, model_id, message)
        }
        json_http::Error::ReqwestTransport { .. } => {
            ModelError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. }
        | json_http::Error::ClientInitialization { .. }
        | json_http::Error::SseUnsupported
        | json_http::Error::HttpStatus { .. }
        | json_http::Error::InvalidSseContentType { .. }
        | json_http::Error::IdleTimeout { .. }
        | json_http::Error::DeadlineExceeded { .. }
        | json_http::Error::SseTransport { .. }
        | json_http::Error::SseDecode { .. } => ModelError::internal(source),
    }
}
