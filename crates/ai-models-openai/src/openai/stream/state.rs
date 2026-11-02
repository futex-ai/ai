//! Direct OpenAI model-event mapping and typed terminal supplementation.

use ai_interface::{Effort, FinishReason, ModelEvent, ModelResult, StructuredOutputSchema};

use crate::openai::response;
use crate::openai::response::types::{ResponsesOutputItem, ResponsesResponse};

use super::{
    text::{ContentKind, TextParts},
    tools::ToolParts,
    types::OpenAiStreamEvent,
};

#[derive(Debug, Default)]
pub(super) struct OpenAiStreamState {
    text: TextParts,
    tools: ToolParts,
    next_part_index: usize,
}

impl OpenAiStreamState {
    pub(super) fn push(&mut self, event: OpenAiStreamEvent) -> Vec<ModelEvent> {
        let mut events = Vec::new();
        match event {
            OpenAiStreamEvent::OutputTextDelta { delta, location }
            | OpenAiStreamEvent::RefusalDelta { delta, location } => {
                self.text.push(
                    location,
                    ContentKind::Text,
                    delta,
                    &mut self.next_part_index,
                    &mut events,
                );
            }
            OpenAiStreamEvent::ReasoningTextDelta { delta, location } => {
                self.text.push(
                    location,
                    ContentKind::Reasoning,
                    delta,
                    &mut self.next_part_index,
                    &mut events,
                );
            }
            OpenAiStreamEvent::ItemAdded { output_index, item } => {
                self.item(output_index, &item, false, &mut events)
            }
            OpenAiStreamEvent::ItemDone { output_index, item } => {
                self.item(output_index, &item, true, &mut events)
            }
            OpenAiStreamEvent::ArgumentsDelta { delta, location } => {
                self.tools.push(location, delta, &mut events)
            }
            OpenAiStreamEvent::ArgumentsDone {
                arguments,
                location,
            } => self.tools.done(location, arguments, &mut events),
            OpenAiStreamEvent::Progress
            | OpenAiStreamEvent::Completed { .. }
            | OpenAiStreamEvent::Incomplete { .. }
            | OpenAiStreamEvent::Failed { .. }
            | OpenAiStreamEvent::Error(_) => {}
        }
        events
    }

    fn item(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
        end: bool,
        events: &mut Vec<ModelEvent>,
    ) {
        self.text
            .supplement_item(output_index, item, &mut self.next_part_index, events);
        self.tools
            .item(output_index, item, end, &mut self.next_part_index, events);
    }

    pub(super) fn complete(
        &mut self,
        catalog_model_id: &str,
        provider_model_id: &str,
        thinking_level: Effort,
        parsed: ResponsesResponse,
        response_schema: Option<&StructuredOutputSchema>,
    ) -> ModelResult<Vec<ModelEvent>> {
        response::check_error(provider_model_id, &parsed)?;
        let mut events = Vec::new();
        for (index, item) in parsed.output.iter().enumerate() {
            self.item(index, item, true, &mut events);
        }
        self.tools.end_all(&mut events);
        if matches!(response::finish_reason(&parsed), FinishReason::ToolCalls) {
            self.tools.validate(provider_model_id)?;
        }
        events.extend(response::terminal_events(
            catalog_model_id,
            provider_model_id,
            thinking_level,
            parsed,
            &self.text.assistant_message(),
            response_schema,
        )?);
        Ok(events)
    }
}
