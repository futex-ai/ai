//! Shared model execution over the injected HTTP boundary.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    DynModelEventStream, Model, ModelError, ModelRequest, ModelResult, ModelSpec, SafeErrorSource,
};
use ai_models_core::{
    ApiKeyScheme, Clock, ErrorContext, ProviderConfig, SystemClock, resolve_controls,
    synthetic_tool_call_scope,
};
use async_trait::async_trait;
use json_http::DynJsonHttpClient;

use super::{
    Dialect, deferred_stream,
    dialect::ReplayRule,
    http_error,
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
    config: ProviderConfig,
    clock: Arc<dyn Clock>,
    deferred_completion: Option<DynDeferredCompletion>,
}

impl CompatModel {
    pub(crate) fn from_config(
        dialect: &'static Dialect,
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            dialect,
            http_client,
            catalog_model_id: spec.text_id(),
            spec,
            config,
            clock: Arc::new(SystemClock),
            deferred_completion: None,
        }
    }

    pub(crate) fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub(crate) fn with_deferred(mut self, completion: DynDeferredCompletion) -> Self {
        self.deferred_completion = Some(completion);
        self
    }

    async fn open_stream(&self, model_request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let context = ErrorContext::new(
            self.dialect.provider,
            self.spec.provider_model_id.as_ref(),
            self.dialect.request_id_headers,
            self.clock.clone(),
        );
        self.config.capture_secrets(&context);
        let controls = match resolve_controls(model_request, &self.spec) {
            Ok(controls) => controls,
            Err(error) => return Err(context.apply_model_error(error)),
        };
        let scope = if self.dialect.replay == ReplayRule::Xai {
            synthetic_tool_call_scope(model_request)
        } else {
            String::new()
        };
        let identity = StreamIdentity {
            error: context.clone(),
            dialect: self.dialect,
            catalog_model_id: self.catalog_model_id.clone(),
            provider_model_id: self.spec.provider_model_id.to_string(),
            thinking_level: controls.effort,
            synthetic_tool_call_scope: scope,
        };
        let request_body = match request::build_request(
            self.dialect,
            self.spec.provider_model_id.as_ref(),
            model_request,
            &controls,
        ) {
            Ok(body) => body,
            Err(error) => return Err(context.apply_model_error(error)),
        };
        if controls.deferred
            && let Some(completion) = self.deferred_completion.as_ref()
        {
            let timeout = controls
                .total_timeout
                .or(self.config.timeouts.total_timeout)
                .or(self.dialect.deferred_timeout)
                .unwrap_or_default();
            let body = completion.complete(&context, request_body, timeout).await?;
            return Ok(deferred_stream::open(
                body,
                identity,
                model_request.response_schema.clone(),
            ));
        }
        let endpoint = self
            .config
            .endpoint(self.dialect.base_url, self.dialect.path);
        let builder = self
            .config
            .apply_headers(self.http_client.post(&endpoint), &endpoint)
            .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
            .timeout(
                controls
                    .total_timeout
                    .or(self.config.timeouts.total_timeout)
                    .unwrap_or(STREAM_TIMEOUT),
            )
            .idle_timeout(
                self.config
                    .timeouts
                    .idle_timeout
                    .unwrap_or(STREAM_IDLE_TIMEOUT),
            );
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => {
                return Err(ModelError::internal(
                    self.dialect.provider,
                    self.spec.provider_model_id.as_ref(),
                    SafeErrorSource::redacted(source),
                ));
            }
        };
        let event_stream = match request.send_sse().await {
            Ok(event_stream) => event_stream,
            Err(source) => return Err(http_error::classify(self.dialect, &context, source)),
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
