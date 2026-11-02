//! Shared model execution over the injected HTTP boundary.

use std::time::Duration;

use ai_interface::{DynModelEventStream, Model, ModelError, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::{
    classify_json_http_error, classify_json_http_stream_error, resolve_controls,
    synthetic_tool_call_scope,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use super::{
    Dialect, deferred_stream,
    dialect::ReplayRule,
    lifecycle::DynDeferredCompletion,
    request,
    stream::{self, StreamIdentity},
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

    async fn open_stream(&self, model_request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let controls = resolve_controls(model_request, &self.spec)?;
        let scope = if self.dialect.replay == ReplayRule::Xai {
            synthetic_tool_call_scope(model_request)
        } else {
            String::new()
        };
        let identity = StreamIdentity {
            dialect: self.dialect,
            catalog_model_id: self.catalog_model_id.clone(),
            provider_model_id: self.spec.provider_model_id.to_string(),
            thinking_level: controls.effort,
            synthetic_tool_call_scope: scope,
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
            return Ok(deferred_stream::open(
                body,
                identity,
                model_request.response_schema.clone(),
            ));
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
        Ok(stream::open(
            event_stream,
            identity,
            model_request.response_schema.clone(),
        ))
    }
}

#[async_trait]
impl Model for CompatModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        self.open_stream(request).await
    }
}
