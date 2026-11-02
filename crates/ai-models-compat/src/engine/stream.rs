//! Pull-based SSE consumption and typed terminal validation.

use std::collections::VecDeque;

use ai_interface::{
    DynModelEventStream, Effort, ErrorKind, ModelError, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{ErrorContext, classify_json_http_stream_error, classify_stream_error};
use async_trait::async_trait;
use json_http::DynJsonHttpSseStream;

use super::{
    dialect::{CompletionContext, Dialect, ReplayRule},
    errors::classify_chat_completions_provider_error,
    response,
    stream_events::{EventParts, terminal_events},
};
use crate::{ChatCompletionsAccumulator, ChatCompletionsResponse, ChatCompletionsStreamError};

pub(crate) trait StreamNormalizer: Send {
    fn normalize_data(
        &mut self,
        data: &str,
        context: &ErrorContext,
        progress: u64,
    ) -> ModelResult<String>;
    fn restore(self: Box<Self>, body: &mut ChatCompletionsResponse) -> Option<String>;
}

pub(super) struct StreamIdentity {
    pub(super) dialect: &'static Dialect,
    /// Bound response facts and the secrets captured for this call.
    pub(super) error: ErrorContext,
    pub(super) catalog_model_id: String,
    pub(super) provider_model_id: String,
    pub(super) thinking_level: Effort,
    pub(super) synthetic_tool_call_scope: String,
}

impl StreamIdentity {
    pub(super) fn context(&self) -> CompletionContext<'_> {
        CompletionContext {
            dialect: self.dialect,
            error: &self.error,
            catalog_model_id: &self.catalog_model_id,
            provider_model_id: &self.provider_model_id,
            thinking_level: self.thinking_level,
            synthetic_tool_call_scope: &self.synthetic_tool_call_scope,
        }
    }
}

pub(super) fn open(
    stream: DynJsonHttpSseStream,
    mut identity: StreamIdentity,
    response_schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    identity.error = identity.error.for_response(stream.headers());
    let normalizer = identity
        .dialect
        .normalizer
        .map(|create| create(&identity.provider_model_id));
    Box::new(CompatStream {
        source: Some(stream),
        identity,
        response_schema,
        accumulator: Some(ChatCompletionsAccumulator::for_model_stream()),
        normalizer,
        parts: EventParts::default(),
        events: VecDeque::new(),
        progress: 0,
    })
}

struct CompatStream {
    source: Option<DynJsonHttpSseStream>,
    identity: StreamIdentity,
    response_schema: Option<StructuredOutputSchema>,
    accumulator: Option<ChatCompletionsAccumulator>,
    normalizer: Option<Box<dyn StreamNormalizer>>,
    parts: EventParts,
    events: VecDeque<ModelEvent>,
    progress: u64,
}

impl CompatStream {
    async fn read(&mut self) -> ModelResult<()> {
        let Some(source) = self.source.as_mut() else {
            return Ok(());
        };
        let context = &self.identity.error;
        let event = match source.next().await {
            Ok(Some(event)) => event,
            Ok(None) if self.identity.dialect.clean_eof => {
                if let Some(accumulator) = self.accumulator.as_mut()
                    && let Err(source) = accumulator.push_data("[DONE]")
                {
                    return Err(classify_stream_error(context, self.progress, &source));
                }
                return self.finish(false);
            }
            Ok(None) => {
                return Err(classify_stream_error(
                    context,
                    self.progress,
                    &ChatCompletionsStreamError::MissingDone,
                ));
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    context,
                    self.progress,
                    source,
                ));
            }
        };
        let data = match self.normalizer.as_mut() {
            Some(normalizer) => normalizer.normalize_data(&event.data, context, self.progress)?,
            None => event.data,
        };
        let Some(accumulator) = self.accumulator.as_mut() else {
            return Ok(());
        };
        match accumulator.push_data_with_chunk(&data) {
            Ok(Some(chunk)) => {
                self.progress = self.progress.saturating_add(1);
                self.events.extend(
                    self.parts
                        .push_chunk(&chunk, self.identity.dialect.replay == ReplayRule::Xai),
                );
                Ok(())
            }
            Ok(None) => self.finish(true),
            Err(ChatCompletionsStreamError::ProviderEvent { error }) => Err(
                classify_chat_completions_provider_error(context, self.progress, error),
            ),
            Err(source) => Err(classify_stream_error(context, self.progress, &source)),
        }
    }

    fn finish(&mut self, explicit_done: bool) -> ModelResult<()> {
        let Some(accumulator) = self.accumulator.take() else {
            return Ok(());
        };
        self.source = None;
        let mut accumulated = match accumulator.finish_model_stream(explicit_done) {
            Ok(accumulated) => accumulated,
            Err(source) => {
                if matches!(
                    source,
                    ChatCompletionsStreamError::MissingToolCallId { .. }
                        | ChatCompletionsStreamError::MissingToolFunctionName { .. }
                        | ChatCompletionsStreamError::MissingLegacyFunctionName
                ) {
                    return Err(ModelError::with_source(
                        self.identity
                            .error
                            .core(ErrorKind::InvalidOutput, source.to_string()),
                        source,
                    ));
                }
                return Err(classify_stream_error(
                    &self.identity.error,
                    self.progress,
                    &source,
                ));
            }
        };
        let terminal_text = self
            .normalizer
            .take()
            .and_then(|normalizer| normalizer.restore(&mut accumulated));
        let legacy = accumulated
            .choices
            .first()
            .and_then(|choice| choice.message.function_call.clone());
        let context = self.identity.context();
        let response =
            response::parse_accumulated(context, accumulated, self.response_schema.as_ref())?;
        self.events
            .extend(self.parts.finish(context, legacy, &response));
        if let Some(delta) = terminal_text.filter(|delta| !delta.is_empty()) {
            self.events.push_back(self.parts.terminal_text(delta));
        }
        self.events.extend(terminal_events(response));
        Ok(())
    }
}

#[async_trait]
impl ModelEventStream for CompatStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Ok(Some(event));
            }
            if self.source.is_none() {
                return Ok(None);
            }
            if let Err(error) = self.read().await {
                self.source = None;
                self.accumulator = None;
                self.normalizer = None;
                self.events.clear();
                return Err(error);
            }
        }
    }
}
