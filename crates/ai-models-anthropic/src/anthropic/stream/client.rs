//! Owned Anthropic event stream and failure classification.

use std::collections::VecDeque;

use ai_interface::{
    DynModelEventStream, Effort, ErrorKind, ModelError, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{ErrorContext, classify_json_http_stream_error, classify_stream_error};
use async_trait::async_trait;
use json_http::DynJsonHttpSseStream;

use crate::anthropic::response;

use super::{
    accumulator::AnthropicStreamAccumulator,
    types::{AnthropicAccumulation, AnthropicProviderErrorKind, AnthropicStreamError},
};

pub(in crate::anthropic) fn open(
    stream: DynJsonHttpSseStream,
    context: ErrorContext,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Option<Effort>,
    response_schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    let context = context.for_response(stream.headers());
    Box::new(AnthropicEventStream {
        context,
        stream: Some(stream),
        accumulator: AnthropicStreamAccumulator::new(provider_model_id.clone()),
        pending: VecDeque::new(),
        events_received: 0,
        catalog_model_id,
        provider_model_id,
        thinking_level,
        response_schema,
    })
}

struct AnthropicEventStream {
    context: ErrorContext,
    stream: Option<DynJsonHttpSseStream>,
    accumulator: AnthropicStreamAccumulator,
    pending: VecDeque<ModelEvent>,
    events_received: u64,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Option<Effort>,
    response_schema: Option<StructuredOutputSchema>,
}

#[async_trait]
impl ModelEventStream for AnthropicEventStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        let result = self.read_event().await;
        if result.is_err() {
            self.stream = None;
            self.pending.clear();
        }
        result
    }
}

impl AnthropicEventStream {
    async fn read_event(&mut self) -> ModelResult<Option<ModelEvent>> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }
            let Some(stream) = self.stream.as_mut() else {
                return Ok(None);
            };
            let event = match stream.next().await {
                Ok(Some(event)) => event,
                Ok(None) => {
                    return Err(classify_stream_error(
                        &self.context,
                        self.events_received,
                        &AnthropicStreamError::UnexpectedEof,
                    ));
                }
                Err(source) => {
                    return Err(classify_json_http_stream_error(
                        &self.context,
                        self.events_received,
                        source,
                    ));
                }
            };
            match self.accumulator.push_data(&event.data) {
                Ok(AnthropicAccumulation::Continue { events }) => {
                    self.events_received = self.events_received.saturating_add(1);
                    self.pending.extend(events);
                }
                Ok(AnthropicAccumulation::Complete(completion)) => {
                    let terminal = response::terminal_events(
                        &self.context,
                        &self.catalog_model_id,
                        &self.provider_model_id,
                        self.thinking_level,
                        completion,
                        self.response_schema.as_ref(),
                    )?;
                    self.stream = None;
                    self.pending.extend(terminal);
                }
                Err(AnthropicStreamError::ProviderEvent { kind, message })
                    if self.events_received == 0 =>
                {
                    return Err(provider_event_error(&self.context, kind, message));
                }
                Err(source @ AnthropicStreamError::InvalidToolInput { .. }) => {
                    return Err(ModelError::with_source(
                        self.context
                            .core(ErrorKind::InvalidOutput, source.to_string()),
                        self.context.source(source),
                    ));
                }
                Err(source @ AnthropicStreamError::SerializeState { .. }) => {
                    return Err(ModelError::with_source(
                        self.context.core(ErrorKind::Internal, "internal error"),
                        self.context.source(source),
                    ));
                }
                Err(source) => {
                    return Err(classify_stream_error(
                        &self.context,
                        self.events_received,
                        &source,
                    ));
                }
            }
        }
    }
}

fn provider_event_error(
    context: &ErrorContext,
    kind: AnthropicProviderErrorKind,
    message: String,
) -> ModelError {
    match kind {
        AnthropicProviderErrorKind::RateLimit => {
            context.model_error(ErrorKind::RateLimited, message)
        }
        AnthropicProviderErrorKind::Overloaded
        | AnthropicProviderErrorKind::Api
        | AnthropicProviderErrorKind::Timeout => context.model_error(ErrorKind::Transient, message),
        AnthropicProviderErrorKind::RequestTooLarge
        | AnthropicProviderErrorKind::InvalidRequest
        | AnthropicProviderErrorKind::Unrecognized => {
            context.model_error(ErrorKind::InvalidRequest, message)
        }
        AnthropicProviderErrorKind::Authentication
        | AnthropicProviderErrorKind::Permission
        | AnthropicProviderErrorKind::Billing => {
            context.model_error(ErrorKind::RejectedKey, message)
        }
        AnthropicProviderErrorKind::NotFound => {
            context.model_error(ErrorKind::UnknownModel, message)
        }
        AnthropicProviderErrorKind::ContextLimit => {
            context.model_error(ErrorKind::ContextLimitExceeded, message)
        }
    }
}
