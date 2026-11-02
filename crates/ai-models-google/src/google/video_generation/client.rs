//! Google video generation transport client.

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use ai_interface::{
    ErrorKind, VideoGenerationError, VideoGenerationRequest, VideoGenerationResult,
};
use ai_models_core::{Clock, DynPollingRuntime, ErrorContext, SystemClock, TokioPollingRuntime};
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, JsonHttpResponse, StaticHeaderAuth};

use crate::google::error_context;

use super::{
    error::{classify_request_error, classify_status},
    request::build_request,
};

const GOOGLE_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Google-backed `ai_interface::VideoGenerator` implementation.
#[derive(Clone)]
pub struct GoogleVideoGenerator {
    http_client: DynJsonHttpClient,
    pub(super) model_id: String,
    auth: DynJsonHttpAuth,
    pub(super) runtime: DynPollingRuntime,
    pub(super) base_url: String,
    pub(super) poll_interval: Duration,
    pub(super) timeout: Duration,
    clock: Arc<dyn Clock>,
    initial_secrets: Vec<String>,
}

impl GoogleVideoGenerator {
    /// Builds a Google video generator from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut model = Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".to_owned(),
                api_key.clone(),
            )]))),
        );
        model.initial_secrets.push(api_key);
        model
    }

    /// Builds a Google video generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_auth_and_runtime(http_client, model_id, auth, Arc::new(TokioPollingRuntime))
    }

    /// Builds a Google video generator with an explicit polling runtime.
    pub fn with_auth_and_runtime(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
        runtime: DynPollingRuntime,
    ) -> Self {
        Self {
            http_client,
            model_id: model_id.into(),
            auth,
            runtime,
            base_url: GOOGLE_API_BASE_URL.to_owned(),
            poll_interval: DEFAULT_POLL_INTERVAL,
            timeout: DEFAULT_TIMEOUT,
            clock: Arc::new(SystemClock),
            initial_secrets: Vec::new(),
        }
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub(super) fn context(&self) -> ErrorContext {
        let context = error_context::new(&self.model_id, self.clock.clone());
        for secret in &self.initial_secrets {
            context.add_secret(secret);
            if let Some(token) = secret.split_whitespace().last() {
                context.add_secret(token);
            }
        }
        context
    }

    /// Overrides the API base URL used for submission, polling, and downloads.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Overrides polling interval and total generation timeout.
    pub fn with_polling(mut self, poll_interval: Duration, timeout: Duration) -> Self {
        self.poll_interval = poll_interval;
        self.timeout = timeout;
        self
    }

    async fn send_json(
        &self,
        context: &ErrorContext,
        request: json_http::JsonHttpRequestBuilder,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(context, source)),
        };
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

    pub(super) async fn submit(
        &self,
        context: &ErrorContext,
        request: &VideoGenerationRequest,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let endpoint = format!(
            "{}/models/{}:predictLongRunning",
            self.base_url, self.model_id
        );
        let builder = match self
            .http_client
            .post(&endpoint)
            .auth(context.capture_auth(self.auth.clone()))
            .timeout(timeout)
            .json(build_request(context, request)?)
        {
            Ok(builder) => builder,
            Err(source) => return Err(classify_request_error(context, source)),
        };
        self.send_json(context, builder).await
    }

    pub(super) async fn retrieve(
        &self,
        context: &ErrorContext,
        operation: &str,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        self.send_json(
            context,
            self.http_client
                .get(&format!("{}/{}", self.base_url, operation))
                .auth(context.capture_auth(self.auth.clone()))
                .timeout(timeout),
        )
        .await
    }

    pub(super) async fn download(
        &self,
        context: &ErrorContext,
        uri: &str,
        timeout: Duration,
    ) -> VideoGenerationResult<Vec<u8>> {
        if !allowed_download_uri(&self.base_url, uri) {
            return Err(VideoGenerationError::new(context.core(
                ErrorKind::InvalidOutput,
                "provider returned a download URI outside the configured API origin",
            )));
        }
        let response = self
            .http_client
            .get(uri)
            .auth(context.capture_auth(self.auth.clone()))
            .timeout(timeout)
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
        let remaining = self
            .timeout
            .saturating_sub(self.runtime.now().saturating_duration_since(started_at));
        if remaining.is_zero() {
            return Err(VideoGenerationError::new(
                context.core(ErrorKind::TimedOut, "generation timed out"),
            ));
        }
        Ok(remaining)
    }
}

fn allowed_download_uri(base_url: &str, uri: &str) -> bool {
    let Some(base_authority) = https_authority(base_url) else {
        return false;
    };
    https_authority(uri) == Some(base_authority)
        && uri
            .strip_prefix("https://")
            .is_some_and(|rest| rest.contains('/'))
        && !uri.bytes().any(|byte| byte.is_ascii_whitespace())
}

fn https_authority(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') || authority.contains('#') {
        return None;
    }
    Some(authority)
}
