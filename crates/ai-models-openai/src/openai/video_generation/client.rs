//! OpenAI video generation transport client.

use std::{
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
    request::{OpenAiVideoApiRequest, build_request},
    response::{JobState, parse_job},
};

const OPENAI_VIDEOS_URL: &str = "https://api.openai.com/v1/videos";
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// OpenAI-backed `ai_interface::VideoGenerator` implementation.
#[derive(Clone)]
pub struct OpenAiVideoGenerator {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    runtime: DynPollingRuntime,
    pub(super) endpoint: String,
    pub(super) poll_interval: Duration,
    pub(super) timeout: Duration,
}

impl OpenAiVideoGenerator {
    /// Builds an OpenAI video generator from an injected client, model id, and API key.
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

    /// Builds an OpenAI video generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_auth_and_runtime(http_client, model_id, auth, Arc::new(TokioPollingRuntime))
    }

    /// Builds an OpenAI video generator with an explicit polling runtime.
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
            endpoint: OPENAI_VIDEOS_URL.to_owned(),
            poll_interval: DEFAULT_POLL_INTERVAL,
            timeout: DEFAULT_TIMEOUT,
        }
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

    async fn send_job_request(
        &self,
        request: OpenAiVideoApiRequest,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let builder = self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .timeout(timeout);
        let response = match request {
            OpenAiVideoApiRequest::Json(body) => match builder.json(body) {
                Ok(request) => request.send_value().await,
                Err(source) => return Err(classify_request_error(source, &self.model_id)),
            },
            OpenAiVideoApiRequest::Multipart(fields) => {
                builder.multipart(fields).send_value().await
            }
        };
        match response {
            Ok(response) => self.successful_json(response),
            Err(source) => Err(classify_request_error(source, &self.model_id)),
        }
    }

    async fn retrieve_job(
        &self,
        job_id: &str,
        timeout: Duration,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let response = self
            .http_client
            .get(&format!("{}/{job_id}", self.endpoint))
            .auth(self.auth.clone())
            .timeout(timeout)
            .send_value()
            .await;
        match response {
            Ok(response) => self.successful_json(response),
            Err(source) => Err(classify_request_error(source, &self.model_id)),
        }
    }

    fn successful_json(
        &self,
        response: JsonHttpResponse<serde_json::Value>,
    ) -> VideoGenerationResult<JsonHttpResponse<serde_json::Value>> {
        if response.status >= 400 {
            return Err(classify_status(
                response.status,
                &self.model_id,
                &response.body,
            ));
        }
        Ok(response)
    }

    async fn download(&self, job_id: &str, timeout: Duration) -> VideoGenerationResult<Vec<u8>> {
        let response = self
            .http_client
            .get(&format!("{}/{job_id}/content", self.endpoint))
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
            return Err(VideoGenerationError::no_video("openai", &self.model_id));
        }
        Ok(response.body)
    }

    fn remaining(&self, started_at: Instant) -> VideoGenerationResult<Duration> {
        let elapsed = self.runtime.now().saturating_duration_since(started_at);
        let remaining = self.timeout.saturating_sub(elapsed);
        if remaining.is_zero() {
            return Err(VideoGenerationError::timed_out("openai", &self.model_id));
        }
        Ok(remaining)
    }
}

#[async_trait]
impl VideoGenerator for OpenAiVideoGenerator {
    async fn generate(
        &self,
        request: &VideoGenerationRequest,
    ) -> VideoGenerationResult<VideoGenerationResponse> {
        let api_request = build_request(&self.model_id, request)?;
        let started_at = self.runtime.now();
        let response = self
            .send_job_request(api_request, self.remaining(started_at)?)
            .await?;
        let (job_id, mut state) = parse_job(&self.model_id, response.body)?;
        while state == JobState::Pending {
            let remaining = self.remaining(started_at)?;
            self.runtime.sleep(remaining.min(self.poll_interval)).await;
            let response = self
                .retrieve_job(&job_id, self.remaining(started_at)?)
                .await?;
            let (retrieved_id, retrieved_state) = parse_job(&self.model_id, response.body)?;
            if retrieved_id != job_id {
                return Err(VideoGenerationError::provider(
                    "openai",
                    &self.model_id,
                    "provider changed the video job id while polling",
                ));
            }
            state = retrieved_state;
        }
        let data = self.download(&job_id, self.remaining(started_at)?).await?;
        let (width, height) = dimensions(request.aspect);
        Ok(VideoGenerationResponse {
            provider: "openai".to_owned(),
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
