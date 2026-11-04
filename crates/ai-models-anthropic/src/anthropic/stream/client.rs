//! Owned Anthropic event stream and failure classification.

use std::collections::VecDeque;

use ai_interface::{
    DynModelEventStream, Effort, ModelError, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{classify_json_http_stream_error, classify_stream_error};
use async_trait::async_trait;
use json_http::DynJsonHttpSseStream;

use crate::anthropic::response;

use super::{
    accumulator::AnthropicStreamAccumulator,
    types::{AnthropicAccumulation, AnthropicProviderErrorKind, AnthropicStreamError},
};

const PROVIDER: &str = "anthropic";

pub(in crate::anthropic) fn open(
    stream: DynJsonHttpSseStream,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Option<Effort>,
    response_schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    Box::new(AnthropicEventStream {
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
                        PROVIDER,
                        &self.provider_model_id,
                        self.events_received,
                        &AnthropicStreamError::UnexpectedEof,
                    ));
                }
                Err(source) => {
                    return Err(classify_json_http_stream_error(
                        PROVIDER,
                        &self.provider_model_id,
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
                    return Err(provider_event_error(&self.provider_model_id, kind, message));
                }
                Err(source) => {
                    return Err(classify_stream_error(
                        PROVIDER,
                        &self.provider_model_id,
                        self.events_received,
                        &source,
                    ));
                }
            }
        }
    }
}

fn provider_event_error(
    model_id: &str,
    kind: AnthropicProviderErrorKind,
    message: String,
) -> ModelError {
    match kind {
        AnthropicProviderErrorKind::RateLimit => {
            ModelError::rate_limited(PROVIDER, model_id, message)
        }
        AnthropicProviderErrorKind::Overloaded | AnthropicProviderErrorKind::Api => {
            ModelError::transient_provider(PROVIDER, model_id, message)
        }
        AnthropicProviderErrorKind::RequestTooLarge
        | AnthropicProviderErrorKind::InvalidRequest
        | AnthropicProviderErrorKind::Authentication
        | AnthropicProviderErrorKind::Permission
        | AnthropicProviderErrorKind::NotFound
        | AnthropicProviderErrorKind::Unrecognized => {
            ModelError::provider(PROVIDER, model_id, message)
        }
    }
}
