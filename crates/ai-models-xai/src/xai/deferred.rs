//! XAI deferred-completion submission and retrieval.

use std::{sync::Arc, time::Duration};

use ai_interface::{ModelError, ModelResult};
use ai_models_core::classify_json_http_error;
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};
use serde::Deserialize;
use serde_json::Value;

use super::request_types::ChatCompletionsRequest;

const CHAT_COMPLETIONS_URL: &str = "https://api.x.ai/v1/chat/completions";
const DEFERRED_COMPLETION_URL: &str = "https://api.x.ai/v1/chat/deferred-completion";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const POLL_TIMEOUT: Duration = Duration::from_secs(30);
const PROVIDER: &str = "xai";

pub(super) type DynDeferredCompletion = Arc<dyn DeferredCompletion>;
pub(super) type DynDeferredRuntime = Arc<dyn DeferredRuntime>;

#[async_trait]
pub(super) trait DeferredCompletion: Send + Sync {
    async fn complete(
        &self,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<Value>;
}

#[cfg_attr(test, unimock::unimock(api = DeferredRuntimeMock))]
#[async_trait]
pub(super) trait DeferredRuntime: Send + Sync {
    fn now(&self) -> tokio::time::Instant;

    async fn sleep(&self, duration: Duration);
}

#[derive(Debug, Default)]
pub(super) struct TokioDeferredRuntime;

#[async_trait]
impl DeferredRuntime for TokioDeferredRuntime {
    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

pub(super) struct XaiDeferredCompletion {
    http_client: DynJsonHttpClient,
    auth: DynJsonHttpAuth,
    model_id: String,
    runtime: DynDeferredRuntime,
}

impl XaiDeferredCompletion {
    pub(super) fn new(
        http_client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
        model_id: impl Into<String>,
        runtime: DynDeferredRuntime,
    ) -> Self {
        Self {
            http_client,
            auth,
            model_id: model_id.into(),
            runtime,
        }
    }

    async fn submit(
        &self,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<String> {
        let builder = self
            .http_client
            .post(CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone())
            .timeout(total_timeout);
        let request = match builder.json(request) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(request_error(source, &self.model_id)),
        };
        if !(200..300).contains(&response.status) {
            return Err(classify_json_http_error(
                PROVIDER,
                &self.model_id,
                response.status,
                &response.body,
            ));
        }
        let accepted: DeferredAccepted = match serde_json::from_value(response.body) {
            Ok(accepted) => accepted,
            Err(source) => {
                return Err(ModelError::provider(
                    PROVIDER,
                    &self.model_id,
                    source.to_string(),
                ));
            }
        };
        validate_request_id(&self.model_id, accepted.request_id)
    }

    async fn retrieve(&self, request_id: &str, timeout: Duration) -> ModelResult<Retrieval> {
        let url = format!("{DEFERRED_COMPLETION_URL}/{request_id}");
        let response = self
            .http_client
            .get(&url)
            .auth(self.auth.clone())
            .timeout(timeout)
            .send_value()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(json_http::Error::Transport { .. } | json_http::Error::Auth { .. }) => {
                return Ok(Retrieval::Retry);
            }
            Err(source) => return Err(ModelError::internal(source)),
        };
        match response.status {
            200 => Ok(Retrieval::Complete(response.body)),
            202 | 429 | 500..=599 => Ok(Retrieval::Retry),
            status => Err(classify_json_http_error(
                PROVIDER,
                &self.model_id,
                status,
                &response.body,
            )),
        }
    }

    fn remaining(
        &self,
        started_at: tokio::time::Instant,
        total_timeout: Duration,
    ) -> ModelResult<Duration> {
        let elapsed = self.runtime.now().saturating_duration_since(started_at);
        let remaining = total_timeout.saturating_sub(elapsed);
        if remaining.is_zero() {
            return Err(timeout_error(&self.model_id));
        }
        Ok(remaining)
    }
}

#[async_trait]
impl DeferredCompletion for XaiDeferredCompletion {
    async fn complete(
        &self,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<Value> {
        let started_at = self.runtime.now();
        let request_id = self.submit(request, total_timeout).await?;
        loop {
            let remaining = self.remaining(started_at, total_timeout)?;
            match self
                .retrieve(&request_id, remaining.min(POLL_TIMEOUT))
                .await?
            {
                Retrieval::Complete(body) => return Ok(body),
                Retrieval::Retry => {
                    let remaining = self.remaining(started_at, total_timeout)?;
                    self.runtime.sleep(remaining.min(POLL_INTERVAL)).await;
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct DeferredAccepted {
    request_id: String,
}

enum Retrieval {
    Complete(Value),
    Retry,
}

fn validate_request_id(model_id: &str, request_id: String) -> ModelResult<String> {
    if request_id.is_empty()
        || request_id.len() > 512
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ModelError::provider(
            PROVIDER,
            model_id,
            "deferred completion returned an invalid request id",
        ));
    }
    Ok(request_id)
}

fn timeout_error(model_id: &str) -> ModelError {
    ModelError::transient_provider(
        PROVIDER,
        model_id,
        "deferred completion exceeded the total call timeout",
    )
}

fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { .. } | json_http::Error::Auth { .. } => {
            ModelError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ModelError::internal(source),
    }
}
