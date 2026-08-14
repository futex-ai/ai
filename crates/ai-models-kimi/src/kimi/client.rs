//! Kimi model construction and request dispatch.

use std::sync::Arc;

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ProviderKind};
use ai_models_core::{ThinkingLevel, classify_json_http_error, resolve_catalog_thinking_level};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use thiserror::Error;

use crate::{KIMI_K3, catalog::known_models};

use super::{request, response};

const KIMI_CHAT_COMPLETIONS_URL: &str = "https://api.moonshot.ai/v1/chat/completions";
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
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    reasoning_effort: KimiReasoningEffort,
    auth: DynJsonHttpAuth,
    endpoint: String,
}

impl KimiModel {
    /// Builds the default max-reasoning Kimi K3 model from an explicit API key.
    pub fn new(http_client: DynJsonHttpClient, api_key: impl Into<String>) -> Self {
        Self::with_auth(
            http_client,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds the default max-reasoning Kimi K3 model from an explicit auth hook.
    pub fn with_auth(http_client: DynJsonHttpClient, auth: DynJsonHttpAuth) -> Self {
        Self {
            http_client,
            catalog_model_id: KIMI_K3.to_owned(),
            provider_model_id: KIMI_K3.to_owned(),
            reasoning_effort: KimiReasoningEffort::Max,
            auth,
            endpoint: KIMI_CHAT_COMPLETIONS_URL.to_owned(),
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
        let reasoning_effort = validate_configuration(&provider_model_id, thinking_level)?;
        Ok(Self {
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id,
            reasoning_effort,
            auth,
            endpoint: KIMI_CHAT_COMPLETIONS_URL.to_owned(),
        })
    }
}

#[async_trait]
impl Model for KimiModel {
    async fn complete(
        &self,
        model_request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ModelError> {
        if let Err(control) = model_request.controls.execution.resolve_deferred(false) {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                control,
            ));
        }
        let response_schema = model_request.response_schema.as_ref();
        let request_body = request::build_request(
            &self.provider_model_id,
            self.reasoning_effort,
            model_request,
        )?;
        let builder = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone());
        let builder = match model_request.controls.execution.total_timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        };
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(request_error(source, &self.provider_model_id)),
        };
        if response.status >= 400 {
            return Err(classify_json_http_error(
                PROVIDER,
                &self.provider_model_id,
                response.status,
                &response.body,
            ));
        }
        response::parse_response(
            &self.catalog_model_id,
            &self.provider_model_id,
            self.reasoning_effort.thinking_level(),
            response.body,
            response_schema,
        )
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
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
            Self::Max => "max",
        }
    }

    fn thinking_level(self) -> ThinkingLevel {
        match self {
            Self::Low => ThinkingLevel::Low,
            Self::High => ThinkingLevel::High,
            Self::Max => ThinkingLevel::Max,
        }
    }
}

pub(super) fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { message } | json_http::Error::Auth { message } => {
            ModelError::transient_provider(PROVIDER, model_id, message)
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ModelError::internal(source),
    }
}
