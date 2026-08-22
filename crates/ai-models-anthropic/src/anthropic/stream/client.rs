//! Anthropic SSE loop and model-error classification.

use ai_interface::{ModelError, ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::{ThinkingLevel, classify_json_http_stream_error, classify_stream_error};
use json_http::DynJsonHttpSseStream;

use super::{
    accumulator::AnthropicStreamAccumulator,
    types::{AnthropicAccumulation, AnthropicProviderErrorKind, AnthropicStreamError},
};
use crate::anthropic::response;

const PROVIDER: &str = "anthropic";

pub(in crate::anthropic) async fn complete(
    mut stream: DynJsonHttpSseStream,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let mut accumulator = AnthropicStreamAccumulator::default();
    let mut events_received = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &AnthropicStreamError::UnexpectedEof,
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
            Ok(AnthropicAccumulation::Continue) => {
                events_received = events_received.saturating_add(1);
            }
            Ok(AnthropicAccumulation::Complete(body)) => {
                return response::parse_response(
                    catalog_model_id,
                    provider_model_id,
                    thinking_level,
                    body,
                    response_schema,
                );
            }
            Err(
                source @ AnthropicStreamError::ProviderEvent {
                    kind: _,
                    message: _,
                },
            ) if events_received > 0 => {
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &source,
                ));
            }
            Err(AnthropicStreamError::ProviderEvent { kind, message }) => {
                return Err(provider_event_error(provider_model_id, kind, message));
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
