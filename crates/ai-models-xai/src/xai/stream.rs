//! xAI synchronous Chat Completions SSE consumption.

use ai_interface::{ModelError, ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::{
    ChatCompletionsAccumulator, ChatCompletionsStreamError, ChatCompletionsStreamStatus,
    ThinkingLevel, classify_chat_completions_provider_error, classify_json_http_stream_error,
    classify_stream_error,
};
use json_http::DynJsonHttpSseStream;

use super::response;

const PROVIDER: &str = "xai";

pub(super) async fn complete(
    mut stream: DynJsonHttpSseStream,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    synthetic_tool_call_scope: &str,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let mut accumulator = ChatCompletionsAccumulator::new();
    let mut events_received = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &ChatCompletionsStreamError::MissingDone,
                ));
            }
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    source,
                ));
            }
        };
        match accumulator.push_data(&event.data) {
            Ok(ChatCompletionsStreamStatus::Chunk) => {
                events_received = events_received.saturating_add(1);
            }
            Ok(ChatCompletionsStreamStatus::Done) => {
                let accumulated = match accumulator.finish() {
                    Ok(accumulated) => accumulated,
                    Err(source) => {
                        return Err(classify_stream_error(
                            PROVIDER,
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
                return response::parse_response(
                    catalog_model_id,
                    provider_model_id,
                    thinking_level,
                    synthetic_tool_call_scope,
                    body,
                    response_schema,
                );
            }
            Err(ChatCompletionsStreamError::ProviderEvent { error }) => {
                return Err(classify_chat_completions_provider_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    error,
                ));
            }
            Err(source) => {
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &source,
                ));
            }
        }
    }
}
