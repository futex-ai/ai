//! Finite events after deferred submission and retrieval complete.

use std::collections::VecDeque;

use ai_interface::{
    ConversationPart, DynModelEventStream, ModelEvent, ModelEventStream, ModelResult,
    StructuredOutputSchema,
};
use async_trait::async_trait;
use json_http::JsonHttpResponse;
use serde_json::Value;

use super::{CompletionContext, response, stream::StreamIdentity, stream_events::terminal_events};

pub(super) fn open(
    response: JsonHttpResponse<Value>,
    mut identity: StreamIdentity,
    schema: Option<StructuredOutputSchema>,
) -> DynModelEventStream {
    identity.error = identity.error.for_response(&response.headers);
    Box::new(DeferredStream {
        body: Some(response.body),
        identity,
        schema,
        events: VecDeque::new(),
    })
}

struct DeferredStream {
    body: Option<Value>,
    identity: StreamIdentity,
    schema: Option<StructuredOutputSchema>,
    events: VecDeque<ModelEvent>,
}

#[async_trait]
impl ModelEventStream for DeferredStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        if let Some(body) = self.body.take() {
            self.events = result_events(self.identity.context(), body, self.schema.as_ref())?;
        }
        Ok(self.events.pop_front())
    }
}

fn result_events(
    context: CompletionContext<'_>,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<VecDeque<ModelEvent>> {
    let (response, content) = response::parse_deferred(context, body, schema)?;
    let mut events = VecDeque::new();
    let mut part_index = 0;
    if let Some(delta) = content.reasoning.filter(|delta| !delta.is_empty()) {
        events.push_back(ModelEvent::ReasoningDelta {
            part_index,
            delta,
            provider_state: None,
        });
        part_index += 1;
    }
    if !response.text().is_empty() {
        events.push_back(ModelEvent::TextDelta {
            part_index,
            delta: response.text(),
            provider_state: None,
        });
        part_index += 1;
    }
    for call in content.calls {
        events.push_back(ModelEvent::ToolCallStart {
            part_index,
            id: call.id.clone(),
            name: call.name,
            operation_id: call.operation_id,
        });
        if !call.arguments.is_empty() {
            events.push_back(ModelEvent::ToolCallArgumentsDelta {
                part_index,
                delta: call.arguments,
            });
        }
        events.push_back(ModelEvent::ToolCallEnd {
            part_index,
            provider_state: response.parts.iter().find_map(|part| match part {
                ConversationPart::ToolCall {
                    call_id,
                    provider_state,
                    ..
                } if *call_id == call.id => provider_state.clone(),
                _ => None,
            }),
        });
        part_index += 1;
    }
    events.extend(terminal_events(response));
    Ok(events)
}
