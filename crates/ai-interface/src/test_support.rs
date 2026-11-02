//! Finite model streams for test fixtures and mock model implementations.

use std::vec::IntoIter;

use async_trait::async_trait;

use crate::{
    DynModelEventStream, ModelAttemptError, ModelError, ModelEvent, ModelEventStream,
    ModelResponse, ModelResult,
};

/// Boxes literal events in a stream that stays exhausted after its last event.
pub fn model_event_stream(events: Vec<ModelEvent>) -> DynModelEventStream {
    Box::new(FiniteModelEventStream {
        events: events.into_iter(),
    })
}

/// Converts a response fixture to events for the real provided completion fold.
pub fn model_response_stream(response: ModelResponse) -> DynModelEventStream {
    let mut events = response
        .attempts
        .into_iter()
        .map(|attempt| ModelEvent::Fallback {
            error: restore_error(attempt.error),
        })
        .collect::<Vec<_>>();
    let mut part_index = 0;
    if !response.assistant_message.is_empty() {
        events.push(ModelEvent::TextDelta {
            part_index,
            delta: response.assistant_message,
        });
        part_index += 1;
    }
    for call in response.tool_calls {
        events.push(ModelEvent::ToolCallStart {
            part_index,
            id: call.id,
            name: call.name,
            operation_id: call.operation_id,
        });
        events.push(ModelEvent::ToolCallArgumentsDelta {
            part_index,
            delta: call.input.to_string(),
        });
        events.push(ModelEvent::ToolCallEnd { part_index });
        part_index += 1;
    }
    events.push(ModelEvent::Usage {
        usage: response.usage,
    });
    events.push(ModelEvent::Finish {
        finish_reason: response.finish_reason,
        provider: response.provider,
        model_id: response.model_id,
        catalog_model_id: response.catalog_model_id,
        thinking_level: response.thinking_level,
        structured_output: response.structured_output,
        provider_context: response.provider_context,
    });
    model_event_stream(events)
}

struct FiniteModelEventStream {
    events: IntoIter<ModelEvent>,
}

#[async_trait]
impl ModelEventStream for FiniteModelEventStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        Ok(self.events.next())
    }
}

fn restore_error(error: ModelAttemptError) -> ModelError {
    match error {
        ModelAttemptError::RateLimited {
            provider,
            model_id,
            message,
        } => ModelError::rate_limited(provider, model_id, message),
        ModelAttemptError::TransientProvider {
            provider,
            model_id,
            message,
        } => ModelError::transient_provider(provider, model_id, message),
        ModelAttemptError::Interrupted {
            provider,
            model_id,
            message,
        } => ModelError::interrupted(provider, model_id, message),
        ModelAttemptError::Provider {
            provider,
            model_id,
            message,
        } => ModelError::provider(provider, model_id, message),
        ModelAttemptError::ContextLimitExceeded {
            provider,
            model_id,
            message,
        } => ModelError::context_limit_exceeded(provider, model_id, message),
        ModelAttemptError::UnsupportedControl {
            provider,
            model_id,
            control,
        } => ModelError::unsupported_control(provider, model_id, control),
        ModelAttemptError::Internal { message } => {
            ModelError::internal(std::io::Error::other(message))
        }
    }
}
