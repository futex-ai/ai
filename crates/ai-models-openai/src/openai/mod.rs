//! OpenAI Responses model client.

mod image_generation;
mod request;
mod request_input;
mod request_types;
mod response;
mod stream;
mod transcription;
mod video_generation;

use std::{sync::Arc, time::Duration};

use ai_interface::{
    Model, ModelCompletionEventSink, ModelControl, ModelError, ModelRequest, ModelResponse,
    NoopModelCompletionEventSink, ProviderKind,
};
use ai_models_core::{
    ThinkingLevel, classify_json_http_stream_error, resolve_catalog_thinking_level,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::catalog::known_models;

pub use image_generation::OpenAiImageGenerator;
pub use transcription::OpenAiAudioTranscriber;
pub use video_generation::OpenAiVideoGenerator;

const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const PROVIDER: &str = "openai";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
/// OpenAI-backed `ai_interface::Model` implementation.
pub struct OpenAiModel {
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
    endpoint: String,
}

impl OpenAiModel {
    /// Builds an OpenAI model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds an OpenAI model from an explicit auth hook.
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

    /// Builds an OpenAI model from catalog metadata and an explicit auth hook.
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
            ProviderKind::OpenAi,
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
            endpoint: OPENAI_RESPONSES_URL.to_owned(),
        }
    }
}

impl OpenAiModel {
    async fn complete_with_sink(
        &self,
        request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> std::result::Result<ModelResponse, ModelError> {
        if let Err(control) = request.controls.execution.resolve_deferred(false) {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                control,
            ));
        }
        if !request.controls.generation.stop_sequences.is_empty() {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                ModelControl::StopSequences,
            ));
        }
        let response_schema = request.response_schema.as_ref();
        let builder = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .timeout(
                request
                    .controls
                    .execution
                    .total_timeout
                    .unwrap_or(COMPLETION_TIMEOUT),
            )
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
            event_sink,
        )
        .await
    }
}

#[async_trait]
impl Model for OpenAiModel {
    async fn complete(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ModelError> {
        self.complete_with_sink(request, &NoopModelCompletionEventSink)
            .await
    }

    async fn complete_with_events(
        &self,
        request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> std::result::Result<ModelResponse, ModelError> {
        if request.response_schema.is_some() {
            return self
                .complete_with_sink(request, &NoopModelCompletionEventSink)
                .await;
        }
        self.complete_with_sink(request, event_sink).await
    }
}

#[cfg(test)]
#[path = "_tests_/openai_tests.rs"]
mod openai_tests;

#[cfg(test)]
#[path = "_tests_/openai_multimodal_tests.rs"]
mod openai_multimodal_tests;

#[cfg(test)]
#[path = "_tests_/openai_response/mod.rs"]
mod openai_response_tests;

#[cfg(test)]
#[path = "_tests_/openai_structured_finish_tests.rs"]
mod openai_structured_finish_tests;

#[cfg(test)]
#[path = "_tests_/openai_reasoning_replay_tests.rs"]
mod openai_reasoning_replay_tests;

#[cfg(test)]
#[path = "_tests_/openai_thinking_tests.rs"]
mod openai_thinking_tests;

#[cfg(test)]
#[path = "_tests_/openai_controls_tests.rs"]
mod openai_controls_tests;

#[cfg(test)]
#[path = "_tests_/openai_streaming_tests.rs"]
mod openai_streaming_tests;

#[cfg(test)]
#[path = "_tests_/openai_stream_error_tests.rs"]
mod openai_stream_error_tests;

#[cfg(test)]
#[path = "_tests_/openai_event_tests.rs"]
mod openai_event_tests;

#[cfg(test)]
#[path = "_tests_/stream_support.rs"]
mod stream_support;
