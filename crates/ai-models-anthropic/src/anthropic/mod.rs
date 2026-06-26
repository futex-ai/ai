//! Anthropic messages model client.

mod request;
mod response;

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse};
use ai_models_core::{ThinkingLevel, classify_json_http_error};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const PROVIDER: &str = "anthropic";

#[derive(Clone)]
/// Anthropic-backed `ai_interface::Model` implementation.
pub struct AnthropicModel {
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
    endpoint: String,
}

impl AnthropicModel {
    /// Builds an Anthropic model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-api-key".to_owned(),
                api_key.into(),
            )]))),
        )
    }

    /// Builds an Anthropic model from an explicit auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        let model_id = model_id.into();
        Self::with_catalog_auth(
            http_client,
            model_id.clone(),
            model_id,
            ThinkingLevel::Disabled,
            auth,
        )
    }

    /// Builds an Anthropic model from catalog metadata and an explicit auth hook.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id: provider_model_id.into(),
            thinking_level,
            auth,
            endpoint: ANTHROPIC_MESSAGES_URL.to_owned(),
        }
    }
}

#[async_trait]
impl Model for AnthropicModel {
    async fn complete(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ModelError> {
        let response_schema = request.response_schema.as_ref();
        let request = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .json(request::build_request(
                &self.provider_model_id,
                self.thinking_level,
                request,
            ))
            .map_err(ModelError::internal)?;
        let response = request
            .send_value()
            .await
            .map_err(|source| request_error(source, &self.provider_model_id))?;
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
            response_schema,
        )
    }
}

fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { .. } | json_http::Error::Auth { .. } => {
            ModelError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ModelError::internal(source),
    }
}

#[cfg(test)]
#[path = "_tests_/anthropic_tests.rs"]
mod anthropic_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_structured_finish_tests.rs"]
mod anthropic_structured_finish_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_multimodal_tests.rs"]
mod anthropic_multimodal_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_thinking_tests.rs"]
mod anthropic_thinking_tests;
