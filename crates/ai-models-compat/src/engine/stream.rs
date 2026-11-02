//! Shared SSE consumption and progress-aware error handling.

use ai_interface::{
    ModelCompletionEventSink, ModelError, ModelResponse, ModelResult, StructuredOutputSchema,
};
use ai_models_core::{classify_json_http_stream_error, classify_stream_error};
use json_http::DynJsonHttpSseStream;

use super::{
    dialect::CompletionContext, errors::classify_chat_completions_provider_error, response,
};
use crate::{ChatCompletionsAccumulator, ChatCompletionsStreamError, ChatCompletionsStreamUpdate};

pub(super) async fn complete(
    mut stream: DynJsonHttpSseStream,
    context: CompletionContext<'_>,
    response_schema: Option<&StructuredOutputSchema>,
    event_sink: &dyn ModelCompletionEventSink,
) -> ModelResult<ModelResponse> {
    let provider_model_id = context.provider_model_id;
    let mut accumulator = ChatCompletionsAccumulator::new();
    let mut events_received = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                return Err(classify_stream_error(
                    context.dialect.provider,
                    provider_model_id,
                    events_received,
                    &ChatCompletionsStreamError::MissingDone,
                ));
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    context.dialect.provider,
                    provider_model_id,
                    events_received,
                    source,
                ));
            }
        };
        match accumulator.push_data_with_deltas(&event.data) {
            Ok(ChatCompletionsStreamUpdate::Chunk { deltas }) => {
                events_received = events_received.saturating_add(1);
                for delta in deltas {
                    event_sink.emit(delta.into()).await;
                }
            }
            Ok(ChatCompletionsStreamUpdate::Done) => {
                let accumulated = match accumulator.finish() {
                    Ok(accumulated) => accumulated,
                    Err(source) => {
                        return Err(classify_stream_error(
                            context.dialect.provider,
                            provider_model_id,
                            events_received,
                            &source,
                        ));
                    }
                };
                let body = match serde_json::to_value(accumulated) {
                    Ok(body) => body,
                    Err(source) => return Err(ModelError::internal(source)),
                };
                return response::parse_response(context, body, response_schema);
            }
            Err(ChatCompletionsStreamError::ProviderEvent { error }) => {
                return Err(classify_chat_completions_provider_error(
                    context.dialect.provider,
                    provider_model_id,
                    events_received,
                    error,
                ));
            }
            Err(source) => {
                return Err(classify_stream_error(
                    context.dialect.provider,
                    provider_model_id,
                    events_received,
                    &source,
                ));
            }
        }
    }
}
