//! Anthropic messages model client.

pub(crate) mod replay;
mod request;
mod request_messages;
mod request_types;
mod response;
mod stream;

use std::{sync::Arc, time::Duration};

use ai_interface::{
    DynModelEventStream, ErrorKind, Model, ModelError, ModelRequest, ModelResult, ModelSpec,
};
use ai_models_core::{
    ApiKeyScheme, Clock, ErrorContext, ProviderConfig, SystemClock,
    classify_json_http_stream_error, resolve_controls,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use crate::PROVIDER_ID;

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const PROVIDER: &str = "anthropic";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const REQUEST_ID_HEADERS: &[&str] = &["request-id", "x-request-id"];

#[derive(Clone)]
/// Anthropic-backed `ai_interface::Model` implementation.
pub struct AnthropicModel {
    http_client: DynJsonHttpClient,
    spec: ModelSpec,
    config: ProviderConfig,
    endpoint: String,
    clock: Arc<dyn Clock>,
}

impl AnthropicModel {
    /// Builds an Anthropic model from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            spec,
            endpoint: config.endpoint(ANTHROPIC_BASE_URL, "/v1/messages"),
            config,
            clock: Arc::new(SystemClock),
        }
    }

    /// Builds an Anthropic model from an explicit API key.
    ///
    /// The spec supplies model facts and the default effort for each call.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        Self::from_config(http_client, spec, ProviderConfig::new(PROVIDER_ID, api_key))
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
        Self::from_config(
            http_client,
            spec,
            ProviderConfig::with_auth(PROVIDER_ID, auth),
        )
    }

    /// Uses an injected wall clock to evaluate response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

#[async_trait]
impl Model for AnthropicModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let context = ErrorContext::new(
            PROVIDER,
            self.spec.provider_model_id.as_ref(),
            REQUEST_ID_HEADERS,
            self.clock.clone(),
        );
        self.config.capture_secrets(&context);
        let controls = match resolve_controls(request, &self.spec) {
            Ok(controls) => controls,
            Err(error) => return Err(context.apply_model_error(error)),
        };
        let response_schema = request.response_schema.as_ref();
        let timeout = controls
            .total_timeout
            .or(self.config.timeouts.total_timeout)
            .unwrap_or(COMPLETION_TIMEOUT);
        let builder = self.config.apply_headers(
            self.http_client
                .post(&self.endpoint)
                .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Header("x-api-key"))))
                .header("anthropic-version", ANTHROPIC_API_VERSION)
                .timeout(timeout)
                .idle_timeout(
                    self.config
                        .timeouts
                        .idle_timeout
                        .unwrap_or(STREAM_IDLE_TIMEOUT),
                ),
            &self.endpoint,
        );
        let builder = match builder.json(request::build_request(
            &context,
            &self.spec.provider_model_id,
            request,
            &controls,
        )?) {
            Ok(builder) => builder,
            Err(source) => {
                return Err(ModelError::with_source(
                    context.core(ErrorKind::Internal, "internal error"),
                    context.source(source),
                ));
            }
        };
        let stream = match builder.send_sse().await {
            Ok(stream) => stream,
            Err(source) => {
                return Err(classify_json_http_stream_error(&context, 0, source));
            }
        };
        Ok(stream::open(
            stream,
            context,
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
#[path = "_tests_/part_replay_tests.rs"]
mod part_replay_tests;

#[cfg(test)]
#[path = "_tests_/legacy_replay_tests.rs"]
mod legacy_replay_tests;

#[cfg(test)]
#[path = "_tests_/anthropic_lifecycle_tests.rs"]
mod anthropic_lifecycle_tests;
#[cfg(test)]
#[path = "_tests_/error_context_tests.rs"]
mod error_context_tests;

#[cfg(test)]
#[path = "_tests_/auth_context_tests.rs"]
mod auth_context_tests;

#[cfg(test)]
#[path = "_tests_/config_tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "_tests_/config_auth_tests.rs"]
mod config_auth_tests;
