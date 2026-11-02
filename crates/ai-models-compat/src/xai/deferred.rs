//! XAI deferred-completion submission and retrieval.

use std::{sync::Arc, time::Duration};

use ai_interface::{ErrorKind, ModelError, ModelResult};
use ai_models_core::{ErrorContext, classify_json_http_error};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, JsonHttpResponse};
use serde::Deserialize;
use serde_json::Value;

use crate::engine::{lifecycle::DeferredCompletion, request_types::ChatCompletionsRequest};

const CHAT_COMPLETIONS_URL: &str = "https://api.x.ai/v1/chat/completions";
const DEFERRED_COMPLETION_URL: &str = "https://api.x.ai/v1/chat/deferred-completion";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const POLL_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) type DynDeferredRuntime = Arc<dyn DeferredRuntime>;

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
    runtime: DynDeferredRuntime,
}

impl XaiDeferredCompletion {
    pub(super) fn new(
        http_client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
        runtime: DynDeferredRuntime,
    ) -> Self {
        Self {
            http_client,
            auth,
            runtime,
        }
    }

    async fn submit(
        &self,
        context: &ErrorContext,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<String> {
        let builder = self
            .http_client
            .post(CHAT_COMPLETIONS_URL)
            .auth(context.capture_auth(self.auth.clone()))
            .timeout(total_timeout);
        let request = match builder.json(request) {
            Ok(request) => request,
            Err(source) => {
                return Err(ModelError::with_source(
                    context.core(ErrorKind::Internal, "internal error"),
                    source,
                ));
            }
        };
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => {
                return Err(ModelError::with_source(
                    context.request_error(&source),
                    source,
                ));
            }
        };
        if !(200..300).contains(&response.status) {
            return Err(classify_json_http_error(
                context,
                response.status,
                &response.headers,
                &response.body,
            ));
        }
        let context = context.for_response(&response.headers);
        let accepted: DeferredAccepted = match serde_json::from_value(response.body) {
            Ok(accepted) => accepted,
            Err(source) => {
                return Err(ModelError::with_source(
                    context.core(ErrorKind::InvalidOutput, source.to_string()),
                    source,
                ));
            }
        };
        validate_request_id(&context, accepted.request_id)
    }

    async fn retrieve(
        &self,
        context: &ErrorContext,
        request_id: &str,
        timeout: Duration,
    ) -> ModelResult<Retrieval> {
        let url = format!("{DEFERRED_COMPLETION_URL}/{request_id}");
        let response = self
            .http_client
            .get(&url)
            .auth(context.capture_auth(self.auth.clone()))
            .timeout(timeout)
            .send_value()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(source)
                if source.is_transport_failure()
                    || matches!(source.cause(), json_http::Error::Auth { .. }) =>
            {
                return Ok(Retrieval::Retry);
            }
            Err(source) => {
                return Err(ModelError::with_source(
                    context.request_error(&source),
                    source,
                ));
            }
        };
        match response.status {
            200 => Ok(Retrieval::Complete(response)),
            202 | 429 | 500..=599 => Ok(Retrieval::Retry),
            status => Err(classify_json_http_error(
                context,
                status,
                &response.headers,
                &response.body,
            )),
        }
    }

    fn remaining(
        &self,
        context: &ErrorContext,
        started_at: tokio::time::Instant,
        total_timeout: Duration,
    ) -> ModelResult<Duration> {
        let elapsed = self.runtime.now().saturating_duration_since(started_at);
        let remaining = total_timeout.saturating_sub(elapsed);
        if remaining.is_zero() {
            return Err(timeout_error(context));
        }
        Ok(remaining)
    }
}

#[async_trait]
impl DeferredCompletion for XaiDeferredCompletion {
    async fn complete(
        &self,
        context: &ErrorContext,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<JsonHttpResponse<Value>> {
        let started_at = self.runtime.now();
        let request_id = self.submit(context, request, total_timeout).await?;
        loop {
            let remaining = self.remaining(context, started_at, total_timeout)?;
            match self
                .retrieve(context, &request_id, remaining.min(POLL_TIMEOUT))
                .await?
            {
                Retrieval::Complete(body) => return Ok(body),
                Retrieval::Retry => {
                    let remaining = self.remaining(context, started_at, total_timeout)?;
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
    Complete(JsonHttpResponse<Value>),
    Retry,
}

fn validate_request_id(context: &ErrorContext, request_id: String) -> ModelResult<String> {
    if request_id.is_empty()
        || request_id.len() > 512
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(context.model_error(
            ErrorKind::InvalidOutput,
            "deferred completion returned an invalid request id",
        ));
    }
    Ok(request_id)
}

fn timeout_error(context: &ErrorContext) -> ModelError {
    context.model_error(
        ErrorKind::Transient,
        "deferred completion exceeded the total call timeout",
    )
}
