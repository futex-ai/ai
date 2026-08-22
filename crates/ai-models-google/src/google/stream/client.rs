//! Google generate-content SSE loop and model-error classification.

use ai_interface::{ModelError, ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::{
    ThinkingLevel, classify_json_http_error, classify_json_http_stream_error, classify_stream_error,
};
use json_http::DynJsonHttpSseStream;

use super::{
    accumulator::GoogleStreamAccumulator,
    types::{GoogleProviderError, GoogleStreamError, GoogleStreamUpdate},
};
use crate::google::response;

const PROVIDER: &str = "google";

pub(in crate::google) async fn complete(
    mut stream: DynJsonHttpSseStream,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    synthetic_tool_call_scope: &str,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let mut accumulator = GoogleStreamAccumulator::default();
    let mut events_received = 0u64;
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                let body = match accumulator.finish_body() {
                    Ok(body) => body,
                    Err(source) => {
                        return Err(classify_stream_error(
                            PROVIDER,
                            provider_model_id,
                            events_received,
                            &source,
                        ));
                    }
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
            Ok(GoogleStreamUpdate::Continue) => {
                events_received = events_received.saturating_add(1);
            }
            Ok(GoogleStreamUpdate::ProviderError(error)) => {
                return Err(provider_error(provider_model_id, events_received, error));
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
