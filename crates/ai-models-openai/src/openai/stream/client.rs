//! OpenAI Responses SSE loop and model-error classification.

use ai_interface::{ModelError, ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::{ThinkingLevel, classify_json_http_stream_error, classify_stream_error};
use json_http::DynJsonHttpSseStream;

use super::types::{OpenAiStreamError, OpenAiStreamEvent};
use crate::openai::response;

const PROVIDER: &str = "openai";

pub(in crate::openai) async fn complete(
    mut stream: DynJsonHttpSseStream,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let mut events_received = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &OpenAiStreamError::UnexpectedEof,
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
        let parsed = match serde_json::from_str::<OpenAiStreamEvent>(&event.data) {
            Ok(parsed) => parsed,
            Err(source) => {
                let failure = OpenAiStreamError::DeserializeEvent { source };
                return Err(classify_stream_error(
                    PROVIDER,
                    provider_model_id,
                    events_received,
                    &failure,
                ));
            }
        };
        match parsed {
            OpenAiStreamEvent::Progress => {
                events_received = events_received.saturating_add(1);
            }
            OpenAiStreamEvent::Completed { response }
            | OpenAiStreamEvent::Incomplete { response } => {
                return response::parse_response(
                    catalog_model_id,
                    provider_model_id,
                    thinking_level,
                    response,
                    response_schema,
                );
            }
            OpenAiStreamEvent::Failed { response } => {
                return Err(event_failure(
                    provider_model_id,
                    events_received,
                    response.into_failure(),
                ));
            }
            OpenAiStreamEvent::Error(error) => {
                return Err(event_failure(
                    provider_model_id,
                    events_received,
                    error.into_failure(),
                ));
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
