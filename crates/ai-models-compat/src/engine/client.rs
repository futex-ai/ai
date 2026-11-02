//! Shared model execution over the injected HTTP boundary.

use std::time::Duration;

use ai_interface::{
    Model, ModelCompletionEventSink, ModelError, ModelRequest, ModelResponse, ModelResult,
    ModelSpec, NoopModelCompletionEventSink,
};
use ai_models_core::{
    classify_json_http_error, classify_json_http_stream_error, resolve_controls,
    synthetic_tool_call_scope,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use super::{
    CompletionContext, Dialect, dialect::ReplayRule, lifecycle::DynDeferredCompletion, request,
    response, stream,
};

const STREAM_TIMEOUT: Duration = Duration::from_secs(3_600);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
pub(crate) struct CompatModel {
    dialect: &'static Dialect,
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    spec: ModelSpec,
    auth: DynJsonHttpAuth,
    deferred_completion: Option<DynDeferredCompletion>,
}

impl CompatModel {
    pub(crate) fn new(
        dialect: &'static Dialect,
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            dialect,
            http_client,
            catalog_model_id: spec.text_id(),
            spec,
            auth,
            deferred_completion: None,
        }
    }

    pub(crate) fn with_deferred(mut self, completion: DynDeferredCompletion) -> Self {
        self.deferred_completion = Some(completion);
        self
    }

    async fn complete_with_sink(
        &self,
        model_request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        let controls = resolve_controls(model_request, &self.spec)?;
        let scope = if self.dialect.replay == ReplayRule::Xai {
            synthetic_tool_call_scope(model_request)
        } else {
            String::new()
        };
        let context = CompletionContext {
            dialect: self.dialect,
            catalog_model_id: &self.catalog_model_id,
            provider_model_id: self.spec.provider_model_id.as_ref(),
            thinking_level: controls.effort,
            synthetic_tool_call_scope: &scope,
        };
        let request_body = request::build_request(
            self.dialect,
            self.spec.provider_model_id.as_ref(),
            model_request,
            &controls,
        )?;
        if controls.deferred
            && let Some(completion) = self.deferred_completion.as_ref()
        {
            let timeout = controls
                .total_timeout
                .or(self.dialect.deferred_timeout)
                .unwrap_or_default();
            let body = completion.complete(request_body, timeout).await?;
            return response::parse_response(context, body, model_request.response_schema.as_ref());
        }
        let builder = self
            .http_client
            .post(self.dialect.endpoint)
            .auth(self.auth.clone())
            .timeout(controls.total_timeout.unwrap_or(STREAM_TIMEOUT))
            .idle_timeout(STREAM_IDLE_TIMEOUT);
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let event_stream = match request.send_sse().await {
            Ok(event_stream) => event_stream,
            Err(json_http::Error::HttpStatus { status, body }) => {
                return Err(match self.dialect.http_error {
                    Some(classify) => classify(self.spec.provider_model_id.as_ref(), status, &body),
                    None => classify_json_http_error(
                        self.dialect.provider,
                        self.spec.provider_model_id.as_ref(),
                        status,
                        &body,
                    ),
                });
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    self.dialect.provider,
                    self.spec.provider_model_id.as_ref(),
                    0,
                    source,
                ));
            }
        };
        stream::complete(
            event_stream,
            context,
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
