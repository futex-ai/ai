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

use ai_interface::{DynModelEventStream, Model, ModelError, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::{classify_json_http_stream_error, resolve_controls};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

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
    spec: ModelSpec,
    auth: DynJsonHttpAuth,
    endpoint: String,
}

impl OpenAiModel {
    /// Builds an OpenAI model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            spec,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds an OpenAI model from an explicit auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth(http_client, spec, auth)
    }

    /// Builds an OpenAI model from catalog metadata and an explicit auth hook.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            spec,
            auth,
            endpoint: OPENAI_RESPONSES_URL.to_owned(),
        }
    }
}

#[async_trait]
impl Model for OpenAiModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let controls = resolve_controls(request, &self.spec)?;
        let response_schema = request.response_schema.as_ref();
        let builder = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .timeout(controls.total_timeout.unwrap_or(COMPLETION_TIMEOUT))
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

#[cfg(test)]
#[path = "_tests_/request_effort_tests.rs"]
mod request_effort_tests;

#[cfg(test)]
#[path = "_tests_/openai_lifecycle_tests.rs"]
mod openai_lifecycle_tests;
