//! Shared model execution over the injected HTTP boundary.

use std::time::Duration;

use ai_interface::{
    Model, ModelCompletionEventSink, ModelError, ModelRequest, ModelResponse, ModelResult,
    NoopModelCompletionEventSink,
};
use ai_models_core::{ThinkingLevel, classify_json_http_error, classify_json_http_stream_error};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use super::{CompletionContext, Dialect, request, stream};

const STREAM_TIMEOUT: Duration = Duration::from_secs(3_600);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
pub(crate) struct CompatModel {
    dialect: &'static Dialect,
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
}

impl CompatModel {
    pub(crate) fn new(
        dialect: &'static Dialect,
        http_client: DynJsonHttpClient,
        catalog_model_id: String,
        provider_model_id: String,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            dialect,
            http_client,
            catalog_model_id,
            provider_model_id,
            thinking_level,
            auth,
        }
    }

    async fn complete_with_sink(
        &self,
        model_request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        if let Err(control) = model_request.controls.execution.resolve_deferred(false) {
            return Err(ModelError::unsupported_control(
                self.dialect.provider,
                &self.provider_model_id,
                control,
            ));
        }
        let request_body = request::build_request(
            self.dialect,
            &self.provider_model_id,
            self.thinking_level,
            model_request,
        )?;
        let builder = self
            .http_client
            .post(self.dialect.endpoint)
            .auth(self.auth.clone())
            .timeout(
                model_request
                    .controls
                    .execution
                    .total_timeout
                    .unwrap_or(STREAM_TIMEOUT),
            )
            .idle_timeout(STREAM_IDLE_TIMEOUT);
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let event_stream = match request.send_sse().await {
            Ok(event_stream) => event_stream,
            Err(json_http::Error::HttpStatus { status, body }) => {
                return Err(match self.dialect.http_error {
                    Some(classify) => classify(&self.provider_model_id, status, &body),
                    None => classify_json_http_error(
                        self.dialect.provider,
                        &self.provider_model_id,
                        status,
                        &body,
                    ),
                });
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    self.dialect.provider,
                    &self.provider_model_id,
                    0,
                    source,
                ));
            }
        };
        stream::complete(
            event_stream,
            CompletionContext {
                dialect: self.dialect,
                catalog_model_id: &self.catalog_model_id,
                provider_model_id: &self.provider_model_id,
                thinking_level: self.thinking_level,
            },
            model_request.response_schema.as_ref(),
            event_sink,
        )
        .await
    }
}

#[async_trait]
impl Model for CompatModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.complete_with_sink(request, &NoopModelCompletionEventSink)
            .await
    }

    async fn complete_with_events(
        &self,
        request: &ModelRequest,
        sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        let sink = if request.response_schema.is_some() {
            &NoopModelCompletionEventSink
        } else {
            sink
        };
        self.complete_with_sink(request, sink).await
    }
}
