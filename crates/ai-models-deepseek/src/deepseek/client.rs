//! DeepSeek model construction and request dispatch.

use std::{sync::Arc, time::Duration};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ModelResult, ProviderKind};
use ai_models_core::{ThinkingLevel, classify_json_http_error, resolve_catalog_thinking_level};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use thiserror::Error;

use crate::{DEEPSEEK_V4_PRO, catalog::known_models};

use super::{request, response};

const DEEPSEEK_CHAT_COMPLETIONS_URL: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_REQUEST_TIMEOUT: Duration = Duration::from_secs(10 * 60);
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
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
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
            http_client,
            catalog_model_id: DEEPSEEK_V4_PRO.to_owned(),
            provider_model_id: DEEPSEEK_V4_PRO.to_owned(),
            thinking_level: ThinkingLevel::High,
            auth,
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
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id,
            thinking_level,
            auth,
        })
    }
}

#[async_trait]
impl Model for DeepSeekModel {
    async fn complete(&self, model_request: &ModelRequest) -> ModelResult<ModelResponse> {
        if let Err(control) = model_request.controls.execution.resolve_deferred(false) {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                control,
            ));
        }
        let request_body =
            request::build_request(&self.provider_model_id, self.thinking_level, model_request)?;
        let builder = self
            .http_client
            .post(DEEPSEEK_CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone());
        let timeout = model_request
            .controls
            .execution
            .total_timeout
            .unwrap_or(DEEPSEEK_REQUEST_TIMEOUT);
        let builder = builder.timeout(timeout);
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
            self.thinking_level,
            response.body,
            model_request.response_schema.as_ref(),
        )
    }
}

fn validate_configuration(
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
) -> DeepSeekConfigurationResult<ThinkingLevel> {
    if !matches!(
        provider_model_id,
        crate::DEEPSEEK_V4_PRO | crate::DEEPSEEK_V4_FLASH
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

pub(super) fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { message } | json_http::Error::Auth { message } => {
            ModelError::transient_provider(PROVIDER, model_id, message)
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ModelError::internal(source),
    }
}
