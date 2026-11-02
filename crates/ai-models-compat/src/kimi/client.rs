//! Kimi public construction and configuration errors.

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

use super::{KIMI_K3, catalog::known_models, dialect::DIALECT};
use crate::engine::CompatModel;

#[cfg(test)]
const PROVIDER: &str = "kimi";

#[derive(Debug, Eq, Error, PartialEq)]
/// Invalid configuration rejected before a Kimi request can be sent.
pub enum KimiConfigurationError {
    /// The initial adapter supports only the Kimi K3 provider model id.
    #[error("[ai_models_kimi/client] unsupported provider model `{provider_model_id}`")]
    UnsupportedProviderModel {
        /// Unsupported upstream model identifier.
        provider_model_id: String,
    },
    /// No supported Kimi K3 thinking level exists at or below the request.
    #[error("[ai_models_kimi/client] no Kimi K3 thinking level at or below `{thinking_level}`")]
    UnsupportedThinkingLevel {
        /// Unsupported normalized thinking-level value.
        thinking_level: &'static str,
    },
}

/// Result alias for Kimi model construction.
pub type KimiConfigurationResult<T> = std::result::Result<T, KimiConfigurationError>;

#[derive(Clone)]
/// Kimi-backed `ai_interface::Model` implementation.
pub struct KimiModel {
    inner: CompatModel,
}

impl KimiModel {
    /// Builds the default max-reasoning Kimi K3 model from an API key.
    pub fn new(http_client: DynJsonHttpClient, api_key: impl Into<String>) -> Self {
        Self::with_auth(
            http_client,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds the default max-reasoning Kimi K3 model from an auth hook.
    pub fn with_auth(http_client: DynJsonHttpClient, auth: DynJsonHttpAuth) -> Self {
        Self {
            inner: CompatModel::new(
                &DIALECT,
                http_client,
                KIMI_K3.to_owned(),
                KIMI_K3.to_owned(),
                ThinkingLevel::Max,
                auth,
            ),
        }
    }

    /// Builds a Kimi model from validated catalog metadata and explicit auth.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> KimiConfigurationResult<Self> {
        let provider_model_id = provider_model_id.into();
        let thinking_level =
            validate_configuration(&provider_model_id, thinking_level)?.thinking_level();
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
impl Model for KimiModel {
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
    requested_thinking_level: ThinkingLevel,
) -> KimiConfigurationResult<KimiReasoningEffort> {
    if provider_model_id != KIMI_K3 {
        return Err(KimiConfigurationError::UnsupportedProviderModel {
            provider_model_id: provider_model_id.to_owned(),
        });
    }
    let models = known_models();
    let Some(thinking_level) = resolve_catalog_thinking_level(
        &models,
        ProviderKind::Kimi,
        provider_model_id,
        requested_thinking_level,
    ) else {
        return Err(KimiConfigurationError::UnsupportedThinkingLevel {
            thinking_level: requested_thinking_level.as_str(),
        });
    };
    if thinking_level == ThinkingLevel::Low {
        return Ok(KimiReasoningEffort::Low);
    }
    if thinking_level == ThinkingLevel::High {
        return Ok(KimiReasoningEffort::High);
    }
    if thinking_level == ThinkingLevel::Max {
        return Ok(KimiReasoningEffort::Max);
    }
    Err(KimiConfigurationError::UnsupportedThinkingLevel {
        thinking_level: thinking_level.as_str(),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum KimiReasoningEffort {
    Low,
    High,
    Max,
}

impl KimiReasoningEffort {
    pub(super) fn thinking_level(self) -> ThinkingLevel {
        match self {
            Self::Low => ThinkingLevel::Low,
            Self::High => ThinkingLevel::High,
            Self::Max => ThinkingLevel::Max,
        }
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
