//! Owned Google event stream and model-error classification.

use std::collections::VecDeque;

use ai_interface::{
    DynModelEventStream, Effort, ErrorKind, ModelError, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{
    ErrorContext, classify_json_http_error, classify_json_http_stream_error, classify_stream_error,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpSseStream, JsonHttpHeaders};

use crate::google::response;

use super::{
    accumulator::GoogleStreamAccumulator,
    types::{GoogleProviderError, GoogleStreamError, GoogleStreamUpdate},
};

pub(in crate::google) fn open(
    stream: DynJsonHttpSseStream,
    context: ErrorContext,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Effort,
    synthetic_tool_call_scope: String,
    response_schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    let response_headers = stream.headers().clone();
    let context = context.for_response(&response_headers);
    Box::new(GoogleEventStream {
        context,
        response_headers,
        stream: Some(stream),
        accumulator: GoogleStreamAccumulator::new(
            synthetic_tool_call_scope,
            provider_model_id.clone(),
        ),
        pending: VecDeque::new(),
        events_received: 0,
        catalog_model_id,
        provider_model_id,
        thinking_level,
        response_schema,
    })
}

struct GoogleEventStream {
    context: ErrorContext,
    response_headers: JsonHttpHeaders,
    stream: Option<DynJsonHttpSseStream>,
    accumulator: GoogleStreamAccumulator,
    pending: VecDeque<ModelEvent>,
    events_received: u64,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Effort,
    response_schema: Option<StructuredOutputSchema>,
}

#[async_trait]
impl ModelEventStream for GoogleEventStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        let result = self.read_event().await;
        if result.is_err() {
            self.stream = None;
            self.pending.clear();
        }
        result
    }
}

impl GoogleEventStream {
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
                    let completion = match self.accumulator.finish() {
                        Ok(completion) => completion,
                        Err(source) => {
                            return Err(stream_error(&self.context, self.events_received, source));
                        }
                    };
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
                    continue;
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
                Ok(GoogleStreamUpdate::Continue { events }) => {
                    self.events_received = self.events_received.saturating_add(1);
                    self.pending.extend(events);
                }
                Ok(GoogleStreamUpdate::ProviderError(error)) => {
                    return Err(provider_error(
                        &self.context,
                        &self.response_headers,
                        self.events_received,
                        error,
                    ));
                }
                Err(source) => {
                    return Err(stream_error(&self.context, self.events_received, source));
                }
            }
        }
    }
}

fn provider_error(
    context: &ErrorContext,
    headers: &JsonHttpHeaders,
    events_received: u64,
    error: GoogleProviderError,
) -> ModelError {
    if events_received == 0 {
        return classify_json_http_error(context, error.code, headers, &error.body());
    }
    let source = GoogleStreamError::ProviderEvent {
        code: error.code,
        status: error.status,
        message: error.message,
    };
    classify_stream_error(context, events_received, &source)
}

fn stream_error(context: &ErrorContext, progress: u64, source: GoogleStreamError) -> ModelError {
    match source {
        source @ GoogleStreamError::SerializeState { .. } => {
            ModelError::with_source(context.core(ErrorKind::Internal, "internal error"), source)
        }
        source => classify_stream_error(context, progress, &source),
    }
}
