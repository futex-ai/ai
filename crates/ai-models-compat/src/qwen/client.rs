//! Qwen public construction and configuration errors.

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

use super::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS, catalog::known_models, dialect::DIALECT};
use crate::engine::CompatModel;

#[cfg(test)]
const PROVIDER: &str = "qwen";

#[derive(Debug, Eq, Error, PartialEq)]
/// Invalid configuration rejected before a Qwen request can be sent.
pub enum QwenConfigurationError {
    /// The adapter supports only current stable Qwen 3.7 provider ids.
    #[error("[ai_models_qwen/client] unsupported provider model `{provider_model_id}`")]
    UnsupportedProviderModel {
        /// Unsupported upstream model identifier.
        provider_model_id: String,
    },
    /// No supported Qwen 3.7 thinking level exists at or below the request.
    #[error("[ai_models_qwen/client] no Qwen 3.7 thinking level at or below `{thinking_level}`")]
    UnsupportedThinkingLevel {
        /// Unsupported normalized thinking-level value.
        thinking_level: &'static str,
    },
}

/// Result alias for Qwen model construction.
pub type QwenConfigurationResult<T> = std::result::Result<T, QwenConfigurationError>;

#[derive(Clone)]
/// Qwen-backed `ai_interface::Model` implementation.
pub struct QwenModel {
    inner: CompatModel,
}

impl QwenModel {
    /// Builds the default high-thinking Qwen 3.7 Plus model from an API key.
    pub fn new(http_client: DynJsonHttpClient, api_key: impl Into<String>) -> Self {
        Self::with_auth(
            http_client,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds the default high-thinking Qwen 3.7 Plus model from an auth hook.
    pub fn with_auth(http_client: DynJsonHttpClient, auth: DynJsonHttpAuth) -> Self {
        Self {
            inner: CompatModel::new(
                &DIALECT,
                http_client,
                QWEN_3_7_PLUS.to_owned(),
                QWEN_3_7_PLUS.to_owned(),
                ThinkingLevel::High,
                auth,
            ),
        }
    }

    /// Builds a Qwen model from validated catalog metadata and explicit auth.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> QwenConfigurationResult<Self> {
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
impl Model for QwenModel {
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
) -> QwenConfigurationResult<ThinkingLevel> {
    if !matches!(
        provider_model_id,
        QWEN_3_7_MAX | QWEN_3_7_PLUS | QWEN_3_7_FLASH
    ) {
        return Err(QwenConfigurationError::UnsupportedProviderModel {
            provider_model_id: provider_model_id.to_owned(),
        });
    }
    let models = known_models();
    match resolve_catalog_thinking_level(
        &models,
        ProviderKind::Qwen,
        provider_model_id,
        thinking_level,
    ) {
        Some(effective) => Ok(effective),
        None => Err(QwenConfigurationError::UnsupportedThinkingLevel {
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
