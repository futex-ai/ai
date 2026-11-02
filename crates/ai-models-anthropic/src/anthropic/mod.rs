//! Anthropic messages model client.

mod request;
mod request_types;
mod response;
mod stream;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{DynModelEventStream, Model, ModelError, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::{classify_json_http_stream_error, resolve_controls};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const PROVIDER: &str = "anthropic";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
/// Anthropic-backed `ai_interface::Model` implementation.
pub struct AnthropicModel {
    http_client: DynJsonHttpClient,
    spec: ModelSpec,
    auth: DynJsonHttpAuth,
    endpoint: String,
}

impl AnthropicModel {
    /// Builds an Anthropic model from an explicit API key.
    ///
    /// The spec supplies model facts and the default effort for each call.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            spec,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-api-key".to_owned(),
                api_key.into(),
            )]))),
        )
    }

    /// Builds an Anthropic model from an explicit auth hook.
    ///
    /// Use a custom spec to call a model outside the provider catalog.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth(http_client, spec, auth)
    }

    /// Builds an Anthropic model from catalog metadata and an explicit auth hook.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            spec,
            auth,
            endpoint: ANTHROPIC_MESSAGES_URL.to_owned(),
        }
    }
}

#[async_trait]
impl Model for AnthropicModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let controls = resolve_controls(request, &self.spec)?;
        let response_schema = request.response_schema.as_ref();
        let timeout = controls.total_timeout.unwrap_or(COMPLETION_TIMEOUT);
        let builder = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .timeout(timeout)
            .idle_timeout(STREAM_IDLE_TIMEOUT);
        let builder = match builder.json(request::build_request(
            &self.spec.provider_model_id,
            request,
            &controls,
        )?) {
            Ok(builder) => builder,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let stream = match builder.send_sse().await {
            Ok(stream) => stream,
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    PROVIDER,
                    &self.spec.provider_model_id,
                    0,
                    source,
                ));
            }
        };
        Ok(stream::open(
            stream,
            self.spec.text_id(),
            self.spec.provider_model_id.to_string(),
            controls.effort,
            response_schema.cloned(),
        ))
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
#[path = "_tests_/anthropic_event_tests.rs"]
mod anthropic_event_tests;

#[cfg(test)]
#[path = "_tests_/stream_support.rs"]
mod stream_support;

#[cfg(test)]
#[path = "_tests_/thinking_tool_choice_tests.rs"]
mod thinking_tool_choice_tests;

#[cfg(test)]
#[path = "_tests_/output_limit_tests.rs"]
mod output_limit_tests;

#[cfg(test)]
#[path = "_tests_/replay_tests.rs"]
mod replay_tests;

#[cfg(test)]
#[path = "_tests_/request_effort_tests.rs"]
mod request_effort_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_lifecycle_tests.rs"]
mod anthropic_lifecycle_tests;
