//! Shared SSE consumption, terminal validation, and progress-aware failures.

use ai_interface::{
    ModelCompletionEvent, ModelCompletionEventSink, ModelError, ModelResponse, ModelResult,
    StructuredOutputSchema,
};
use ai_models_core::{classify_json_http_stream_error, classify_stream_error};
use json_http::DynJsonHttpSseStream;
use serde_json::Value;

use super::{
    dialect::CompletionContext, errors::classify_chat_completions_provider_error, response,
};
use crate::{ChatCompletionsAccumulator, ChatCompletionsStreamError, ChatCompletionsStreamUpdate};

pub(crate) trait StreamNormalizer: Send {
    fn normalize_data(&mut self, data: &str, model_id: &str, progress: u64) -> ModelResult<String>;
    fn restore(self: Box<Self>, body: &mut Value) -> Result<Option<String>, serde_json::Error>;
}

pub(super) async fn complete(
    mut stream: DynJsonHttpSseStream,
    context: CompletionContext<'_>,
    response_schema: Option<&StructuredOutputSchema>,
    event_sink: &dyn ModelCompletionEventSink,
) -> ModelResult<ModelResponse> {
    let provider = context.dialect.provider;
    let model_id = context.provider_model_id;
    let mut accumulator = ChatCompletionsAccumulator::new();
    let mut normalizer = context.dialect.normalizer.map(|create| create(model_id));
    let mut progress = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) if context.dialect.clean_eof => {
                if let Err(source) = accumulator.push_data("[DONE]") {
                    return Err(classify_stream_error(provider, model_id, progress, &source));
                }
                return finish(
                    accumulator,
                    normalizer,
                    context,
                    response_schema,
                    progress,
                    event_sink,
                )
                .await;
            }
            Ok(None) => {
                return Err(classify_stream_error(
                    provider,
                    model_id,
                    progress,
                    &ChatCompletionsStreamError::MissingDone,
                ));
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    provider, model_id, progress, source,
                ));
            }
        };
        let data = match normalizer.as_mut() {
            Some(normalizer) => normalizer.normalize_data(&event.data, model_id, progress)?,
            None => event.data,
        };
        match accumulator.push_data_with_deltas(&data) {
            Ok(ChatCompletionsStreamUpdate::Chunk { deltas }) => {
                progress = progress.saturating_add(1);
                for delta in deltas {
                    event_sink.emit(delta.into()).await;
                }
            }
            Ok(ChatCompletionsStreamUpdate::Done) => {
                return finish(
                    accumulator,
                    normalizer,
                    context,
                    response_schema,
                    progress,
                    event_sink,
                )
                .await;
            }
            Err(ChatCompletionsStreamError::ProviderEvent { error }) => {
                return Err(classify_chat_completions_provider_error(
                    provider, model_id, progress, error,
                ));
            }
            Err(source) => {
                return Err(classify_stream_error(provider, model_id, progress, &source));
            }
        }
    }
}

async fn finish(
    accumulator: ChatCompletionsAccumulator,
    normalizer: Option<Box<dyn StreamNormalizer>>,
    context: CompletionContext<'_>,
    schema: Option<&StructuredOutputSchema>,
    progress: u64,
    sink: &dyn ModelCompletionEventSink,
) -> ModelResult<ModelResponse> {
    let accumulated = match accumulator.finish() {
        Ok(accumulated) => accumulated,
        Err(source) => {
            return Err(classify_stream_error(
                context.dialect.provider,
                context.provider_model_id,
                progress,
                &source,
            ));
        }
    };
    let mut body = match serde_json::to_value(accumulated) {
        Ok(body) => body,
        Err(source) => return Err(ModelError::internal(source)),
    };
    let terminal_text = match normalizer {
        Some(normalizer) => match normalizer.restore(&mut body) {
            Ok(text) => text,
            Err(source) => return Err(ModelError::internal(source)),
        },
        None => None,
    };
    let response = response::parse_response(context, body, schema)?;
    if let Some(delta) = terminal_text.filter(|delta| !delta.is_empty()) {
        sink.emit(ModelCompletionEvent::AssistantTextDelta { delta })
            .await;
    }
    Ok(response)
}
