//! Owned Google event stream and model-error classification.

use std::collections::VecDeque;

use ai_interface::{
    DynModelEventStream, Effort, ModelError, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{
    classify_json_http_error, classify_json_http_stream_error, classify_stream_error,
};
use async_trait::async_trait;
use json_http::DynJsonHttpSseStream;

use crate::google::response;

use super::{
    accumulator::GoogleStreamAccumulator,
    types::{GoogleProviderError, GoogleStreamError, GoogleStreamUpdate},
};

const PROVIDER: &str = "google";

pub(in crate::google) fn open(
    stream: DynJsonHttpSseStream,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Option<Effort>,
    synthetic_tool_call_scope: String,
    response_schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    Box::new(GoogleEventStream {
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
    stream: Option<DynJsonHttpSseStream>,
    accumulator: GoogleStreamAccumulator,
    pending: VecDeque<ModelEvent>,
    events_received: u64,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Option<Effort>,
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
                            return Err(classify_stream_error(
                                PROVIDER,
                                &self.provider_model_id,
                                self.events_received,
                                &source,
                            ));
                        }
                    };
                    let terminal = response::terminal_events(
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
                        PROVIDER,
                        &self.provider_model_id,
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
                        &self.provider_model_id,
                        self.events_received,
                        error,
                    ));
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

fn provider_error(model_id: &str, events_received: u64, error: GoogleProviderError) -> ModelError {
    if events_received == 0 {
        return classify_json_http_error(PROVIDER, model_id, error.code, &error.body());
    }
    let source = GoogleStreamError::ProviderEvent {
        code: error.code,
        status: error.status,
        message: error.message,
    };
    classify_stream_error(PROVIDER, model_id, events_received, &source)
}
