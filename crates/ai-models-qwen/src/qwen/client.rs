//! Qwen model construction and request dispatch.

use std::sync::Arc;

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ModelResult};
use ai_models_core::{ThinkingLevel, classify_json_http_error};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use serde_json::Value;
use thiserror::Error;

use crate::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS};

use super::{request, response};

const QWEN_CHAT_COMPLETIONS_URL: &str =
    "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";
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
    /// Qwen 3.7 catalog variants support high or disabled thinking.
    #[error("[ai_models_qwen/client] unsupported Qwen 3.7 thinking level `{thinking_level}`")]
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
        validate_configuration(&provider_model_id, thinking_level)?;
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
        let request_body =
            request::build_request(&self.provider_model_id, self.thinking_level, model_request)?;
        let request = match self
            .http_client
            .post(QWEN_CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone())
            .json(request_body)
        {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let http_response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(request_error(source, &self.provider_model_id)),
        };
        if http_response.status >= 400 {
            return Err(classify_qwen_http_error(
                &self.provider_model_id,
                http_response.status,
                &http_response.body,
            ));
        }
        response::parse_response(
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            http_response.body,
            model_request.response_schema.as_ref(),
        )
    }
}

fn validate_configuration(
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
) -> QwenConfigurationResult<()> {
    if !matches!(
        provider_model_id,
        QWEN_3_7_MAX | QWEN_3_7_PLUS | QWEN_3_7_FLASH
    ) {
        return Err(QwenConfigurationError::UnsupportedProviderModel {
            provider_model_id: provider_model_id.to_owned(),
        });
    }
    if !matches!(
        thinking_level,
        ThinkingLevel::Disabled | ThinkingLevel::High
    ) {
        return Err(QwenConfigurationError::UnsupportedThinkingLevel {
            thinking_level: thinking_level.as_str(),
        });
    }
    Ok(())
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

pub(super) fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { message } | json_http::Error::Auth { message } => {
            ModelError::transient_provider(PROVIDER, model_id, message)
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ModelError::internal(source),
    }
}
