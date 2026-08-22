//! Qwen model construction and request dispatch.

use std::{sync::Arc, time::Duration};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ModelResult, ProviderKind};
use ai_models_core::{
    ThinkingLevel, classify_json_http_error, classify_json_http_stream_error,
    resolve_catalog_thinking_level,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use serde_json::Value;
use thiserror::Error;

use crate::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS, catalog::known_models};

use super::{request, stream};

const QWEN_CHAT_COMPLETIONS_URL: &str =
    "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";
const COMPLETION_STREAM_TIMEOUT: Duration = Duration::from_secs(3_600);
const COMPLETION_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
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
/// QwenCloud-backed `ai_interface::Model` implementation.
pub struct QwenModel {
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
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
            http_client,
            catalog_model_id: QWEN_3_7_PLUS.to_owned(),
            provider_model_id: QWEN_3_7_PLUS.to_owned(),
            thinking_level: ThinkingLevel::High,
            auth,
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
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id,
            thinking_level,
            auth,
        })
    }
}

#[async_trait]
impl Model for QwenModel {
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
            .post(QWEN_CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone());
        let timeout = model_request
            .controls
            .execution
            .total_timeout
            .unwrap_or(COMPLETION_STREAM_TIMEOUT);
        let builder = builder
            .timeout(timeout)
            .idle_timeout(COMPLETION_STREAM_IDLE_TIMEOUT);
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let event_stream = match request.send_sse().await {
            Ok(event_stream) => event_stream,
            Err(json_http::Error::HttpStatus { status, body }) => {
                return Err(classify_qwen_http_error(
                    &self.provider_model_id,
                    status,
                    &body,
                ));
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    PROVIDER,
                    &self.provider_model_id,
                    0,
                    source,
                ));
            }
        };
        stream::complete(
            event_stream,
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            model_request.response_schema.as_ref(),
        )
        .await
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

pub(super) fn classify_qwen_http_error(model_id: &str, status: u16, body: &Value) -> ModelError {
    if status == 400
        && let Some(message) = error_message(body)
        && message.starts_with("Range of input length should be [1,")
    {
        return ModelError::context_limit_exceeded(
            PROVIDER,
            model_id,
            format!("HTTP {status}: {message}"),
        );
    }
    classify_json_http_error(PROVIDER, model_id, status, body)
}

fn error_message(body: &Value) -> Option<&str> {
    body.get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
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
