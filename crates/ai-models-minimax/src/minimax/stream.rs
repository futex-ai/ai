//! MiniMax SSE normalization and Chat Completions accumulation.

use ai_interface::{ModelError, ModelResponse, StructuredOutputSchema};
use ai_models_core::{
    ChatCompletionsAccumulator, ChatCompletionsStreamError, ChatCompletionsStreamStatus,
    ThinkingLevel, classify_chat_completions_provider_error, classify_json_http_stream_error,
    classify_stream_error,
};
use json_http::DynJsonHttpSseStream;

use super::{
    response,
    stream_normalizer::{MiniMaxNormalizer, MiniMaxStreamError, NormalizedEvent},
};

const PROVIDER: &str = "minimax";

pub(super) async fn complete(
    mut stream: DynJsonHttpSseStream,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    response_schema: Option<&StructuredOutputSchema>,
) -> std::result::Result<ModelResponse, ModelError> {
    let mut accumulator = ChatCompletionsAccumulator::new();
    let mut normalizer = MiniMaxNormalizer::new(provider_model_id);
    let mut events_received = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                if let Err(source) = accumulator.push_data("[DONE]") {
                    return Err(classify_stream_error(
                        PROVIDER,
                        provider_model_id,
                        events_received,
                        &source,
                    ));
                }
                return finish(
                    accumulator,
                    normalizer,
                    catalog_model_id,
                    provider_model_id,
                    thinking_level,
                    response_schema,
                    events_received,
                );
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
        let normalized = match normalizer.normalize(&event.data) {
            Ok(normalized) => normalized,
            Err(source) => {
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &source,
                ));
            }
        };
        let data = match normalized {
            NormalizedEvent::Done => "[DONE]".to_owned(),
            NormalizedEvent::Chunk(body) => {
                let provider_error =
                    match response::stream_base_response_error(provider_model_id, &body) {
                        Ok(error) => error,
                        Err(source) => {
                            let source = MiniMaxStreamError::DeserializeChunk { source };
                            return Err(classify_stream_error(
                                PROVIDER,
                                provider_model_id,
                                events_received,
                                &source,
                            ));
                        }
                    };
                if let Some(error) = provider_error {
                    if events_received == 0 {
                        return Err(error);
                    }
                    return Err(ModelError::interrupted(
                        PROVIDER,
                        provider_model_id,
                        error.to_string(),
                    ));
                }
                body.to_string()
            }
        };
        match accumulator.push_data(&data) {
            Ok(ChatCompletionsStreamStatus::Chunk) => {
                events_received = events_received.saturating_add(1);
            }
            Ok(ChatCompletionsStreamStatus::Done) => {
                return finish(
                    accumulator,
                    normalizer,
                    catalog_model_id,
                    provider_model_id,
                    thinking_level,
                    response_schema,
                    events_received,
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

fn finish(
    accumulator: ChatCompletionsAccumulator,
    normalizer: MiniMaxNormalizer,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    response_schema: Option<&StructuredOutputSchema>,
    events_received: u64,
) -> std::result::Result<ModelResponse, ModelError> {
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
    let mut body = match serde_json::to_value(accumulated) {
        Ok(body) => body,
        Err(source) => return Err(ModelError::internal(source)),
    };
    if let Err(source) = normalizer.restore_reasoning_details(&mut body) {
        return Err(ModelError::internal(source));
    }
    response::parse_response(
        catalog_model_id,
        provider_model_id,
        thinking_level,
        body,
        response_schema,
    )
}
