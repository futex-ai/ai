//! Direct Responses events, replay state, and anonymous terminal supplementation.

use ai_interface::{Effort, FinishReason, ModelEvent, ModelResult, StructuredOutputSchema};

use crate::openai::response;
use crate::openai::response::types::{ResponsesOutputItem, ResponsesResponse};

use super::{
    replay::item_state,
    text::{ContentKind, TextParts},
    tools::ToolParts,
    types::OpenAiStreamEvent,
};

#[derive(Debug)]
pub(super) struct OpenAiStreamState {
    model_id: String,
    text: TextParts,
    tools: ToolParts,
    next_part_index: usize,
    anonymous: Vec<OpenAiStreamEvent>,
}

impl OpenAiStreamState {
    pub(super) fn new(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_owned(),
            text: TextParts::default(),
            tools: ToolParts::new(model_id),
            next_part_index: 0,
            anonymous: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, event: OpenAiStreamEvent) -> ModelResult<Vec<ModelEvent>> {
        if matches!(event, OpenAiStreamEvent::Progress) {
            return Ok(Vec::new());
        }
        if anonymous_delta(&event) {
            self.anonymous.push(event);
            return Ok(Vec::new());
        }
        let mut events = self.flush_anonymous()?;
        self.push_identified(event, &mut events)?;
        Ok(events)
    }

    fn push_identified(
        &mut self,
        event: OpenAiStreamEvent,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        match event {
            OpenAiStreamEvent::OutputTextDelta { delta, location }
            | OpenAiStreamEvent::RefusalDelta { delta, location } => self.text.push(
                location,
                ContentKind::Text,
                delta,
                &mut self.next_part_index,
                events,
            ),
            OpenAiStreamEvent::ReasoningTextDelta { delta, location } => self.text.push(
                location,
                ContentKind::Reasoning,
                delta,
                &mut self.next_part_index,
                events,
            ),
            OpenAiStreamEvent::ItemAdded { output_index, item } => {
                self.item(output_index, &item, false, false, events)?
            }
            OpenAiStreamEvent::ItemDone { output_index, item } => {
                self.item(output_index, &item, true, false, events)?
            }
            OpenAiStreamEvent::ArgumentsDelta { delta, location } => {
                self.tools.push(location, delta, events)
            }
            OpenAiStreamEvent::ArgumentsDone {
                arguments,
                location,
            } => self.tools.done(location, arguments, events)?,
            _ => {}
        }
        Ok(())
    }

    fn item(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
        end: bool,
        preserve_message: bool,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        let state = item_state(item, &self.model_id, preserve_message)?;
        self.text
            .supplement_item(output_index, item, state, &mut self.next_part_index, events);
        self.tools
            .item(output_index, item, end, &mut self.next_part_index, events)
    }

    pub(super) fn flush_anonymous(&mut self) -> ModelResult<Vec<ModelEvent>> {
        let mut events = Vec::new();
        for event in std::mem::take(&mut self.anonymous) {
            self.push_identified(event, &mut events)?;
        }
        Ok(events)
    }

    pub(super) fn complete(
        &mut self,
        catalog_model_id: &str,
        provider_model_id: &str,
        thinking_level: Option<Effort>,
        parsed: ResponsesResponse,
        response_schema: Option<&StructuredOutputSchema>,
    ) -> ModelResult<Vec<ModelEvent>> {
        let text = self.text.clone();
        let next_part_index = self.next_part_index;
        let anonymous = copy_anonymous(&self.anonymous);
        let result = self.complete_output(
            catalog_model_id,
            provider_model_id,
            thinking_level,
            parsed,
            response_schema,
        );
        if result.is_err() {
            self.text = text;
            self.next_part_index = next_part_index;
            self.anonymous = anonymous;
        }
        result
    }

    fn complete_output(
        &mut self,
        catalog_model_id: &str,
        provider_model_id: &str,
        thinking_level: Option<Effort>,
        parsed: ResponsesResponse,
        response_schema: Option<&StructuredOutputSchema>,
    ) -> ModelResult<Vec<ModelEvent>> {
        response::check_error(provider_model_id, &parsed)?;
        let preserve_message = matches!(response::finish_reason(&parsed), FinishReason::ToolCalls);
        let mut events = Vec::new();
        if self.next_part_index == 0
            && matches!(
                parsed.output.first(),
                Some(ResponsesOutputItem::Reasoning { .. })
            )
        {
            for (index, item) in parsed.output.iter().enumerate() {
                self.release_for_item(index, item, &mut events)?;
                self.item(index, item, true, preserve_message, &mut events)?;
            }
            events.extend(self.flush_anonymous()?);
        } else {
            events.extend(self.flush_anonymous()?);
            for (index, item) in parsed.output.iter().enumerate() {
                self.item(index, item, true, preserve_message, &mut events)?;
            }
        }
        self.tools.end_all(&mut events)?;
        if preserve_message {
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

    fn release_for_item(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        let reasoning = matches!(item, ResponsesOutputItem::Reasoning { .. });
        if !reasoning && !matches!(item, ResponsesOutputItem::Message { .. }) {
            return Ok(());
        }
        let mut remaining = Vec::new();
        for mut event in std::mem::take(&mut self.anonymous) {
            let matches = match &mut event {
                OpenAiStreamEvent::ReasoningTextDelta { location, .. } if reasoning => {
                    location.item.output_index = Some(output_index);
                    true
                }
                OpenAiStreamEvent::OutputTextDelta { location, .. }
                | OpenAiStreamEvent::RefusalDelta { location, .. }
                    if !reasoning =>
                {
                    location.item.output_index = Some(output_index);
                    true
                }
                _ => false,
            };
            if matches {
                self.push_identified(event, events)?;
            } else {
                remaining.push(event);
            }
        }
        self.anonymous = remaining;
        Ok(())
    }
}

fn anonymous_delta(event: &OpenAiStreamEvent) -> bool {
    match event {
        OpenAiStreamEvent::OutputTextDelta { location, .. }
        | OpenAiStreamEvent::RefusalDelta { location, .. }
        | OpenAiStreamEvent::ReasoningTextDelta { location, .. } => {
            location.item.output_index.is_none() && location.item.item_id.is_none()
        }
        _ => false,
    }
}

fn copy_anonymous(events: &[OpenAiStreamEvent]) -> Vec<OpenAiStreamEvent> {
    events
        .iter()
        .filter_map(|event| match event {
            OpenAiStreamEvent::OutputTextDelta { delta, location } => {
                Some(OpenAiStreamEvent::OutputTextDelta {
                    delta: delta.clone(),
                    location: location.clone(),
                })
            }
            OpenAiStreamEvent::RefusalDelta { delta, location } => {
                Some(OpenAiStreamEvent::RefusalDelta {
                    delta: delta.clone(),
                    location: location.clone(),
                })
            }
            OpenAiStreamEvent::ReasoningTextDelta { delta, location } => {
                Some(OpenAiStreamEvent::ReasoningTextDelta {
                    delta: delta.clone(),
                    location: location.clone(),
                })
            }
            _ => None,
        })
        .collect()
}
