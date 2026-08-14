//! Google video generation transport client.

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use ai_interface::{
    GeneratedVideo, ModelUsage, VideoGenerationAspect, VideoGenerationError,
    VideoGenerationRequest, VideoGenerationResponse, VideoGenerationResult, VideoGenerator,
};
use ai_models_core::{DynPollingRuntime, TokioPollingRuntime};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, JsonHttpResponse, StaticHeaderAuth};

use super::{
    error::{classify_request_error, classify_status},
    request::build_request,
    response::{OperationState, parse_operation},
};

const GOOGLE_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Google-backed `ai_interface::VideoGenerator` implementation.
#[derive(Clone)]
pub struct GoogleVideoGenerator {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    runtime: DynPollingRuntime,
    pub(super) base_url: String,
    pub(super) poll_interval: Duration,
    pub(super) timeout: Duration,
}

impl GoogleVideoGenerator {
    /// Builds a Google video generator from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".to_owned(),
                api_key.into(),
            )]))),
        )
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
        }
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
        request: json_http::JsonHttpRequestBuilder,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(source, &self.model_id)),
        };
        if response.status >= 400 {
            return Err(classify_status(
                response.status,
                &self.model_id,
                &response.body,
            ));
        }
        Ok(response)
    }

    async fn submit(
        &self,
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
            .auth(self.auth.clone())
            .timeout(timeout)
            .json(build_request(request)?)
        {
            Ok(builder) => builder,
            Err(source) => return Err(classify_request_error(source, &self.model_id)),
        };
        self.send_json(builder).await
    }

    async fn retrieve(
        &self,
        operation: &str,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        self.send_json(
            self.http_client
                .get(&format!("{}/{}", self.base_url, operation))
                .auth(self.auth.clone())
                .timeout(timeout),
        )
        .await
    }

    async fn download(&self, uri: &str, timeout: Duration) -> VideoGenerationResult<Vec<u8>> {
        if !allowed_download_uri(&self.base_url, uri) {
            return Err(VideoGenerationError::provider(
                "google",
                &self.model_id,
                "provider returned a download URI outside the configured API origin",
            ));
        }
        let response = self
            .http_client
            .get(uri)
            .auth(self.auth.clone())
            .timeout(timeout)
            .send_bytes()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(source, &self.model_id)),
        };
        if response.status >= 400 {
            return Err(classify_status(
                response.status,
                &self.model_id,
                &serde_json::Value::String(String::from_utf8_lossy(&response.body).into_owned()),
            ));
        }
        if response.body.is_empty() {
            return Err(VideoGenerationError::no_video("google", &self.model_id));
        }
        Ok(response.body)
    }

    fn remaining(&self, started_at: Instant) -> VideoGenerationResult<Duration> {
        let remaining = self
            .timeout
            .saturating_sub(self.runtime.now().saturating_duration_since(started_at));
        if remaining.is_zero() {
            return Err(VideoGenerationError::timed_out("google", &self.model_id));
        }
        Ok(remaining)
    }
}

#[async_trait]
impl VideoGenerator for GoogleVideoGenerator {
    async fn generate(
        &self,
        request: &VideoGenerationRequest,
    ) -> VideoGenerationResult<VideoGenerationResponse> {
        let started_at = self.runtime.now();
        let response = self.submit(request, self.remaining(started_at)?).await?;
        let (operation, mut state) = parse_operation(&self.model_id, response.body)?;
        while state == OperationState::Pending {
            let remaining = self.remaining(started_at)?;
            self.runtime.sleep(remaining.min(self.poll_interval)).await;
            let response = self
                .retrieve(&operation, self.remaining(started_at)?)
                .await?;
            let (retrieved_operation, retrieved_state) =
                parse_operation(&self.model_id, response.body)?;
            if retrieved_operation != operation {
                return Err(VideoGenerationError::provider(
                    "google",
                    &self.model_id,
                    "provider changed the operation name while polling",
                ));
            }
            state = retrieved_state;
        }
        let OperationState::Completed { download_uri } = state else {
            return Err(VideoGenerationError::no_video("google", &self.model_id));
        };
        let data = self
            .download(&download_uri, self.remaining(started_at)?)
            .await?;
        let (width, height) = dimensions(request.aspect);
        Ok(VideoGenerationResponse {
            provider: "google".to_owned(),
            model_id: self.model_id.clone(),
            video: GeneratedVideo {
                data,
                mime_type: "video/mp4".to_owned(),
                duration_seconds: request.duration.seconds(),
                width,
                height,
            },
            usage: ModelUsage::default(),
        })
    }
}

fn dimensions(aspect: VideoGenerationAspect) -> (u16, u16) {
    match aspect {
        VideoGenerationAspect::Landscape => (1280, 720),
        VideoGenerationAspect::Portrait => (720, 1280),
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
