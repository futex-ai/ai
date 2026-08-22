//! Anthropic messages model client.

mod request;
mod response;
mod stream;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ProviderKind};
use ai_models_core::{
    ThinkingLevel, classify_json_http_stream_error, resolve_catalog_thinking_level,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::catalog::{find_known_model, known_models};

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const PROVIDER: &str = "anthropic";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

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
    ///
    /// Known catalog ids retain their provider model id and thinking level.
    /// Unknown ids are treated as direct provider ids with thinking disabled.
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
    ///
    /// Known catalog ids retain their provider model id and thinking level.
    /// Unknown ids are treated as direct provider ids with thinking disabled.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        let catalog_model_id = model_id.into();
        let model = find_known_model(&catalog_model_id);
        let provider_model_id = model.map_or_else(
            || catalog_model_id.clone(),
            |(provider_model_id, _)| provider_model_id.to_owned(),
        );
        let thinking_level = model.map_or(ThinkingLevel::Disabled, |(_, level)| level);
        Self::with_catalog_auth(
            http_client,
            catalog_model_id,
            provider_model_id,
            thinking_level,
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
        let provider_model_id = provider_model_id.into();
        let models = known_models();
        let thinking_level = resolve_catalog_thinking_level(
            &models,
            ProviderKind::Anthropic,
            &provider_model_id,
            thinking_level,
        )
        .unwrap_or(thinking_level);
        Self {
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id,
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
        if let Err(control) = request.controls.execution.resolve_deferred(false) {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                control,
            ));
        }
        let response_schema = request.response_schema.as_ref();
        let timeout = request
            .controls
            .execution
            .total_timeout
            .unwrap_or(COMPLETION_TIMEOUT);
        let builder = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .timeout(timeout)
            .idle_timeout(STREAM_IDLE_TIMEOUT);
        let builder = match builder.json(request::build_request(
            &self.provider_model_id,
            self.thinking_level,
            request,
        )?) {
            Ok(builder) => builder,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let stream = match builder.send_sse().await {
            Ok(stream) => stream,
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
            stream,
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            response_schema,
        )
        .await
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

#[cfg(test)]
#[path = "_tests_/anthropic_controls_tests.rs"]
mod anthropic_controls_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_streaming_tests.rs"]
mod anthropic_streaming_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_stream_error_tests.rs"]
mod anthropic_stream_error_tests;

#[cfg(test)]
#[path = "_tests_/stream_support.rs"]
mod stream_support;
