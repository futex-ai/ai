//! Google Gemini generate-content model client.

mod contents;
mod error_context;
mod image_generation;
mod legacy;
mod request;
mod request_types;
mod response;
mod state;
mod stream;
mod thinking;
mod tool_config;
mod video_generation;

pub use image_generation::GoogleImageGenerator;
pub use legacy::GoogleLegacyProviderConverter;
pub use video_generation::GoogleVideoGenerator;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    DynModelEventStream, ErrorKind, Model, ModelError, ModelRequest, ModelResult, ModelSpec,
};
use ai_models_core::{
    Clock, SystemClock, classify_json_http_stream_error, resolve_controls,
    synthetic_tool_call_scope,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

const GOOGLE_GENERATE_CONTENT_URL_PREFIX: &str =
    "https://generativelanguage.googleapis.com/v1beta/models";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
/// Google-backed `ai_interface::Model` implementation.
pub struct GoogleModel {
    http_client: DynJsonHttpClient,
    spec: ModelSpec,
    auth: DynJsonHttpAuth,
    clock: Arc<dyn Clock>,
    initial_secrets: Vec<String>,
}

impl GoogleModel {
    /// Builds a Google model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut model = Self::with_auth(
            http_client,
            spec,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".to_owned(),
                api_key.clone(),
            )]))),
        );
        model.initial_secrets.push(api_key);
        model
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
            clock: Arc::new(SystemClock),
            initial_secrets: Vec::new(),
        }
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "{GOOGLE_GENERATE_CONTENT_URL_PREFIX}/{}:streamGenerateContent?alt=sse",
            self.spec.provider_model_id
        )
    }
}

#[async_trait]
impl Model for GoogleModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let context = error_context::new(&self.spec.provider_model_id, self.clock.clone());
        for secret in &self.initial_secrets {
            context.add_secret(secret);
            if let Some(token) = secret.split_whitespace().last() {
                context.add_secret(token);
            }
        }
        let controls = match resolve_controls(request, &self.spec) {
            Ok(controls) => controls,
            Err(error) => return Err(context.apply_model_error(error)),
        };
        let response_schema = request.response_schema.as_ref();
        let synthetic_tool_call_scope = synthetic_tool_call_scope(request);
        let builder = self
            .http_client
            .post(&self.endpoint())
            .auth(context.capture_auth(self.auth.clone()))
            .timeout(controls.total_timeout.unwrap_or(COMPLETION_TIMEOUT))
            .idle_timeout(STREAM_IDLE_TIMEOUT);
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
                    source,
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
            synthetic_tool_call_scope,
            response_schema.cloned(),
        ))
    }
}

#[cfg(test)]
#[path = "_tests_/error_context_support.rs"]
mod error_context_support;
#[cfg(test)]
#[path = "_tests_/google/mod.rs"]
mod google_tests;

#[cfg(test)]
#[path = "_tests_/error_context_tests.rs"]
mod error_context_tests;

#[cfg(test)]
#[path = "_tests_/auth_context_tests.rs"]
mod auth_context_tests;
