//! Google Gemini generate-content model client.

mod image_generation;
mod request;
mod request_types;
mod response;
mod stream;
mod thinking;
mod tool_config;
mod video_generation;

pub use image_generation::GoogleImageGenerator;
pub use video_generation::GoogleVideoGenerator;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    Model, ModelCompletionEventSink, ModelError, ModelRequest, ModelResponse, ModelSpec,
    NoopModelCompletionEventSink,
};
use ai_models_core::{
    classify_json_http_stream_error, resolve_controls, synthetic_tool_call_scope,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

const GOOGLE_GENERATE_CONTENT_URL_PREFIX: &str =
    "https://generativelanguage.googleapis.com/v1beta/models";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const PROVIDER: &str = "google";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
/// Google-backed `ai_interface::Model` implementation.
pub struct GoogleModel {
    http_client: DynJsonHttpClient,
    spec: ModelSpec,
    auth: DynJsonHttpAuth,
}

impl GoogleModel {
    /// Builds a Google model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            spec,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".to_owned(),
                api_key.into(),
            )]))),
        )
    }

    /// Builds a Google model from an explicit auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth(http_client, spec, auth)
    }

    /// Builds a Google model from catalog metadata and an explicit auth hook.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            spec,
            auth,
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "{GOOGLE_GENERATE_CONTENT_URL_PREFIX}/{}:streamGenerateContent?alt=sse",
            self.spec.provider_model_id
        )
    }
}

impl GoogleModel {
    async fn complete_with_sink(
        &self,
        request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> std::result::Result<ModelResponse, ModelError> {
        let controls = resolve_controls(request, &self.spec)?;
        let response_schema = request.response_schema.as_ref();
        let synthetic_tool_call_scope = synthetic_tool_call_scope(request);
        let builder = self
            .http_client
            .post(&self.endpoint())
            .auth(self.auth.clone())
            .timeout(controls.total_timeout.unwrap_or(COMPLETION_TIMEOUT))
            .idle_timeout(STREAM_IDLE_TIMEOUT);
        let builder = match builder.json(request::build_request(
            &self.spec.provider_model_id,
            request,
            &controls,
        )) {
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
        stream::complete(
            stream,
            &self.spec.text_id(),
            &self.spec.provider_model_id,
            controls.effort,
            &synthetic_tool_call_scope,
            response_schema,
            event_sink,
        )
        .await
    }
}

#[async_trait]
impl Model for GoogleModel {
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
#[path = "_tests_/google/mod.rs"]
mod google_tests;
