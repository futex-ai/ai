//! OpenAI video generation transport client.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use ai_interface::{ErrorKind, ModelSpec, VideoGenerationError, VideoGenerationResult};
use ai_models_core::{
    ApiKeyScheme, Clock, DynPollingRuntime, ErrorContext, ProviderConfig, SystemClock,
    TokioPollingRuntime,
};
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, JsonHttpResponse};

use crate::openai::error_context;
use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

use super::{
    error::{classify_request_error, classify_status},
    request::OpenAiVideoApiRequest,
};

const OPENAI_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// OpenAI-backed `ai_interface::VideoGenerator` implementation.
#[derive(Clone)]
pub struct OpenAiVideoGenerator {
    http_client: DynJsonHttpClient,
    pub(super) model_id: String,
    config: ProviderConfig,
    pub(super) runtime: DynPollingRuntime,
    pub(super) endpoint: String,
    pub(super) poll_interval: Duration,
    pub(super) timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl OpenAiVideoGenerator {
    /// Builds an OpenAI video generator from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            model_id: spec.provider_model_id.into_owned(),
            runtime: Arc::new(TokioPollingRuntime),
            endpoint: config.endpoint(OPENAI_BASE_URL, "/v1/videos"),
            poll_interval: config
                .timeouts
                .poll_interval
                .unwrap_or(DEFAULT_POLL_INTERVAL),
            timeout: config.timeouts.total_timeout.unwrap_or(DEFAULT_TIMEOUT),
            config,
            clock: Arc::new(SystemClock),
        }
    }

    /// Uses an injected runtime for generation deadlines and polling.
    pub fn with_runtime(mut self, runtime: DynPollingRuntime) -> Self {
        self.runtime = runtime;
        self
    }

    /// Builds an OpenAI video generator from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::from_config(
            http_client,
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::new(PROVIDER_ID, api_key),
        )
    }

    /// Builds an OpenAI video generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::from_config(
            http_client,
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::with_auth(PROVIDER_ID, auth),
        )
    }

    /// Builds an OpenAI video generator with an explicit polling runtime.
    pub fn with_auth_and_runtime(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
        runtime: DynPollingRuntime,
    ) -> Self {
        Self::with_auth(http_client, model_id, auth).with_runtime(runtime)
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub(super) fn context(&self) -> ErrorContext {
        let context = error_context::new(&self.model_id, self.clock.clone());
        self.config.capture_secrets(&context);
        context
    }

    /// Overrides the videos endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Overrides polling interval and total generation timeout.
    pub fn with_polling(mut self, poll_interval: Duration, timeout: Duration) -> Self {
        self.poll_interval = poll_interval;
        self.timeout = timeout;
        self
    }

    pub(super) async fn send_job_request(
        &self,
        context: &ErrorContext,
        request: OpenAiVideoApiRequest,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let builder = self.config.apply_headers(
            self.http_client
                .post(&self.endpoint)
                .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                .timeout(timeout),
        );
        let response = match request {
            OpenAiVideoApiRequest::Json(body) => match builder.json(body) {
                Ok(request) => request.send_value().await,
                Err(source) => return Err(classify_request_error(context, source)),
            },
            OpenAiVideoApiRequest::Multipart(fields) => {
                builder.multipart(fields).send_value().await
            }
        };
        match response {
            Ok(response) => self.successful_json(context, response),
            Err(source) => Err(classify_request_error(context, source)),
        }
    }

    pub(super) async fn retrieve_job(
        &self,
        context: &ErrorContext,
        job_id: &str,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let response = self
            .config
            .apply_headers(
                self.http_client
                    .get(&format!("{}/{job_id}", self.endpoint))
                    .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                    .timeout(timeout),
            )
            .send_value()
            .await;
        match response {
            Ok(response) => self.successful_json(context, response),
            Err(source) => Err(classify_request_error(context, source)),
        }
    }

    fn successful_json(
        &self,
        context: &ErrorContext,
        response: JsonHttpResponse<serde_json::Value>,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        if response.status >= 400 {
            return Err(classify_status(
                context,
                response.status,
                &response.headers,
                &response.body,
            ));
        }
        Ok(response)
    }

    pub(super) async fn download(
        &self,
        context: &ErrorContext,
        job_id: &str,
        timeout: Duration,
    ) -> VideoGenerationResult<Vec<u8>> {
        let response = self
            .config
            .apply_headers(
                self.http_client
                    .get(&format!("{}/{job_id}/content", self.endpoint))
                    .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                    .timeout(timeout),
            )
            .send_bytes()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(context, source)),
        };
        if response.status >= 400 {
            return Err(classify_status(
                context,
                response.status,
                &response.headers,
                &serde_json::Value::String(String::from_utf8_lossy(&response.body).into_owned()),
            ));
        }
        let context = context.for_response(&response.headers);
        if response.body.is_empty() {
            return Err(VideoGenerationError::NoVideo {
                core: Box::new(context.core(ErrorKind::InvalidOutput, "no video returned")),
            });
        }
        Ok(response.body)
    }

    pub(super) fn remaining(
        &self,
        context: &ErrorContext,
        started_at: Instant,
    ) -> VideoGenerationResult<Duration> {
        let elapsed = self.runtime.now().saturating_duration_since(started_at);
        let remaining = self.timeout.saturating_sub(elapsed);
        if remaining.is_zero() {
            return Err(VideoGenerationError::new(
                context.core(ErrorKind::TimedOut, "generation timed out"),
            ));
        }
        Ok(remaining)
    }
}
