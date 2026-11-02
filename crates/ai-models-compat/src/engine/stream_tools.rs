//! Direct modern tool-call events with identity retained across fragments.

use ai_interface::ModelEvent;

use crate::chat_completions::ChatCompletionsToolCallDelta;

pub(super) struct ToolPart {
    part_index: usize,
    id: Option<String>,
    name: Option<String>,
}

impl ToolPart {
    pub(super) fn new(part_index: usize) -> Self {
        Self {
            part_index,
            id: None,
            name: None,
        }
    }

    pub(super) fn part_index(&self) -> usize {
        self.part_index
    }

    pub(super) fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub(super) fn push(&mut self, call: &ChatCompletionsToolCallDelta) -> Option<ModelEvent> {
        if let Some(id) = call.id.as_ref() {
            self.id = Some(id.clone());
        }
        if let Some(name) = call.function.name.as_ref() {
            self.name = Some(name.clone());
        }
        call.function
            .arguments
            .as_ref()
            .filter(|delta| !delta.is_empty())
            .map(|delta| ModelEvent::ToolCallArgumentsDelta {
                part_index: self.part_index,
                delta: delta.clone(),
            })
    }

    pub(super) fn start(&self) -> Option<ModelEvent> {
        Some(ModelEvent::ToolCallStart {
            part_index: self.part_index,
            id: self.id.clone()?,
            name: self.name.clone()?,
            operation_id: None,
        })
    }

    pub(super) fn remove_prior_part(&mut self, removed: usize) {
        if self.part_index > removed {
            self.part_index -= 1;
        }
    }
}

pub(super) fn remove_part(event: &mut ModelEvent, removed: usize) -> bool {
    let part_index = match event {
        ModelEvent::TextDelta { part_index, .. }
        | ModelEvent::ReasoningDelta { part_index, .. }
        | ModelEvent::ToolCallStart { part_index, .. }
        | ModelEvent::ToolCallArgumentsDelta { part_index, .. }
        | ModelEvent::ToolCallEnd { part_index, .. } => part_index,
        _ => return false,
    };
    if *part_index == removed {
        return true;
    }
    if *part_index > removed {
        *part_index -= 1;
    }
    false
}
