//! Owned OpenAI Responses stream and failure classification.

use std::collections::VecDeque;

use ai_interface::{
    DynModelEventStream, Effort, ModelError, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{classify_json_http_stream_error, classify_stream_error};
use async_trait::async_trait;
use json_http::DynJsonHttpSseStream;

use super::{
    state::OpenAiStreamState,
    types::{OpenAiStreamError, OpenAiStreamEvent},
};

const PROVIDER: &str = "openai";

pub(in crate::openai) fn open(
    stream: DynJsonHttpSseStream,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Effort,
    response_schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    Box::new(OpenAiEventStream {
        stream: Some(stream),
        state: OpenAiStreamState::new(&provider_model_id),
        deferred_error: None,
        pending: VecDeque::new(),
        events_received: 0,
        catalog_model_id,
        provider_model_id,
        thinking_level,
        response_schema,
    })
}

struct OpenAiEventStream {
    stream: Option<DynJsonHttpSseStream>,
    state: OpenAiStreamState,
    pending: VecDeque<ModelEvent>,
    events_received: u64,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: Effort,
    response_schema: Option<StructuredOutputSchema>,
    deferred_error: Option<ModelError>,
}

#[async_trait]
impl ModelEventStream for OpenAiEventStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        match self.read_event().await {
            Ok(event) => Ok(event),
            Err(error) => {
                self.stream = None;
                self.pending.clear();
                self.pending.extend(self.state.flush_anonymous()?);
                if self.pending.is_empty() {
                    return Err(error);
                }
                self.deferred_error = Some(error);
                Ok(self.pending.pop_front())
            }
        }
    }
}

impl OpenAiEventStream {
    async fn read_event(&mut self) -> ModelResult<Option<ModelEvent>> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }
            if let Some(error) = self.deferred_error.take() {
                return Err(error);
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
                        &OpenAiStreamError::UnexpectedEof,
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
            let parsed = match serde_json::from_str::<OpenAiStreamEvent>(&event.data) {
                Ok(parsed) => parsed,
                Err(source) => {
                    return Err(classify_stream_error(
                        PROVIDER,
                        &self.provider_model_id,
                        self.events_received,
                        &OpenAiStreamError::DeserializeEvent { source },
                    ));
                }
            };
            match parsed {
                OpenAiStreamEvent::Completed { response }
                | OpenAiStreamEvent::Incomplete { response } => {
                    let events = self.state.complete(
                        &self.catalog_model_id,
                        &self.provider_model_id,
                        self.thinking_level,
                        response,
                        self.response_schema.as_ref(),
                    )?;
                    self.stream = None;
                    self.pending.extend(events);
                }
                OpenAiStreamEvent::Failed { response } => {
                    return Err(event_failure(
                        &self.provider_model_id,
                        self.events_received,
                        response.into_failure(),
                    ));
                }
                OpenAiStreamEvent::Error(error) => {
                    return Err(event_failure(
                        &self.provider_model_id,
                        self.events_received,
                        error.into_failure(),
                    ));
                }
                event => {
                    self.events_received = self.events_received.saturating_add(1);
                    self.pending.extend(self.state.push(event)?);
                }
            }
        }
    }
}

fn event_failure(model_id: &str, events_received: u64, failure: OpenAiStreamError) -> ModelError {
    if events_received > 0 {
        return classify_stream_error(PROVIDER, model_id, events_received, &failure);
    }
    let code = failure.code().map(str::to_owned);
    let message = failure
        .provider_message()
        .unwrap_or_else(|| "OpenAI stream failed".to_owned());
    match code.as_deref() {
        Some("rate_limit_error" | "rate_limit_exceeded") => {
            ModelError::rate_limited(PROVIDER, model_id, message)
        }
        Some("internal_error" | "server_error" | "overloaded" | "temporarily_unavailable") => {
            ModelError::transient_provider(PROVIDER, model_id, message)
        }
        _ => ModelError::provider(PROVIDER, model_id, message),
    }
}
