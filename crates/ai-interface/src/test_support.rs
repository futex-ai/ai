//! Finite model streams for test fixtures and mock model implementations.

use std::vec::IntoIter;

use async_trait::async_trait;

use crate::{
    ConversationPart, DynModelEventStream, ModelError, ModelEvent, ModelEventStream, ModelResponse,
    ModelResult,
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
            emitted_events: attempt.emitted_events,
            error: ModelError::from_attempt(attempt),
        })
        .collect::<Vec<_>>();
    for (part_index, part) in response.parts.into_iter().enumerate() {
        match part {
            ConversationPart::Text {
                text,
                provider_state,
            } => events.push(ModelEvent::TextDelta {
                part_index,
                delta: text,
                provider_state,
            }),
            ConversationPart::Reasoning {
                text,
                provider_state,
            } => events.push(ModelEvent::ReasoningDelta {
                part_index,
                delta: text,
                provider_state,
            }),
            ConversationPart::ToolCall {
                call_id,
                tool_name,
                arguments,
                operation_id,
                provider_state,
            } => {
                events.push(ModelEvent::ToolCallStart {
                    part_index,
                    id: call_id,
                    name: tool_name,
                    operation_id,
                });
                events.push(ModelEvent::ToolCallArgumentsDelta {
                    part_index,
                    delta: arguments.to_string(),
                });
                events.push(ModelEvent::ToolCallEnd {
                    part_index,
                    provider_state,
                });
            }
            _ => {}
        }
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
