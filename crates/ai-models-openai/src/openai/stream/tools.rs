//! Native function-call identity, argument fragments, and completion.

use std::collections::BTreeMap;

use ai_interface::{ModelError, ModelEvent, ModelResult};
use ai_models_core::parse_tool_call_arguments;

use crate::openai::replay_state::{FunctionCallState, ReplayState};
use crate::openai::response::types::ResponsesOutputItem;

use super::types::{ItemKey, ItemLocation};

#[derive(Debug)]
struct ToolPart {
    model_id: String,
    call_id: String,
    name: String,
    item_id: Option<String>,
    part_index: usize,
    arguments: String,
    ended: bool,
}

#[derive(Debug, Default)]
struct PendingArguments {
    fragments: Vec<String>,
    final_arguments: Option<String>,
}

#[derive(Debug)]
pub(super) struct ToolParts {
    model_id: String,
    parts: BTreeMap<usize, ToolPart>,
    pending: BTreeMap<ItemKey, PendingArguments>,
}

impl ToolParts {
    pub(super) fn new(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_owned(),
            parts: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }

    pub(super) fn item(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
        end: bool,
        next_part_index: &mut usize,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        let ResponsesOutputItem::FunctionCall {
            id,
            call_id,
            name,
            arguments,
        } = item
        else {
            return Ok(());
        };
        let part = self.parts.entry(output_index).or_insert_with(|| {
            let part_index = *next_part_index;
            *next_part_index = next_part_index.saturating_add(1);
            events.push(ModelEvent::ToolCallStart {
                part_index,
                id: call_id.clone(),
                name: name.clone(),
                operation_id: None,
            });
            ToolPart {
                model_id: self.model_id.clone(),
                call_id: call_id.clone(),
                name: name.clone(),
                item_id: id.clone(),
                part_index,
                arguments: String::new(),
                ended: false,
            }
        });
        if id.is_some() {
            part.item_id = id.clone();
        }
        let pending = self
            .pending
            .remove(&ItemKey::Index(output_index))
            .or_else(|| {
                id.as_ref()
                    .and_then(|id| self.pending.remove(&ItemKey::Id(id.clone())))
            })
            .or_else(|| self.pending.remove(&ItemKey::Anonymous));
        if let Some(pending) = pending {
            for fragment in pending.fragments {
                part.push(fragment, events);
            }
            if let Some(arguments) = pending.final_arguments {
                part.complete(&arguments, true, events)?;
            }
        }
        part.complete(arguments, end, events)
    }

    pub(super) fn push(
        &mut self,
        location: ItemLocation,
        delta: String,
        events: &mut Vec<ModelEvent>,
    ) {
        if delta.is_empty() {
            return;
        }
        if let Some(index) = self.find(&location) {
            if let Some(part) = self.parts.get_mut(&index) {
                part.push(delta, events);
            }
        } else {
            self.pending
                .entry(location.key())
                .or_default()
                .fragments
                .push(delta);
        }
    }

    pub(super) fn done(
        &mut self,
        location: ItemLocation,
        arguments: String,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        if let Some(index) = self.find(&location) {
            if let Some(part) = self.parts.get_mut(&index) {
                part.complete(&arguments, true, events)?;
            }
        } else {
            self.pending
                .entry(location.key())
                .or_default()
                .final_arguments = Some(arguments);
        }
        Ok(())
    }

    fn find(&self, location: &ItemLocation) -> Option<usize> {
        if let Some(index) = location.output_index {
            return self.parts.contains_key(&index).then_some(index);
        }
        if let Some(id) = &location.item_id {
            return self
                .parts
                .iter()
                .find(|(_, part)| part.item_id.as_ref() == Some(id))
                .map(|(index, _)| *index);
        }
        self.parts
            .iter()
            .filter(|(_, part)| !part.ended)
            .max_by_key(|(_, part)| part.part_index)
            .map(|(index, _)| *index)
    }

    pub(super) fn end_all(&mut self, events: &mut Vec<ModelEvent>) -> ModelResult<()> {
        for part in self.parts.values_mut() {
            part.complete("", true, events)?;
        }
        Ok(())
    }

    pub(super) fn validate(&self, model_id: &str) -> ModelResult<()> {
        if self.parts.is_empty() {
            return Err(ModelError::provider(
                "openai",
                model_id,
                "tool_calls finish had no complete tool call",
            ));
        }
        for part in self.parts.values() {
            parse_tool_call_arguments("openai", model_id, &part.arguments)?;
        }
        Ok(())
    }
}

impl ToolPart {
    fn push(&mut self, delta: String, events: &mut Vec<ModelEvent>) {
        if self.ended || delta.is_empty() {
            return;
        }
        self.arguments.push_str(&delta);
        events.push(ModelEvent::ToolCallArgumentsDelta {
            part_index: self.part_index,
            delta,
        });
    }

    fn complete(
        &mut self,
        arguments: &str,
        end: bool,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        if self.ended {
            return Ok(());
        }
        if let Some(suffix) = arguments.strip_prefix(&self.arguments) {
            self.push(suffix.to_owned(), events);
        }
        if end {
            self.ended = true;
            events.push(ModelEvent::ToolCallEnd {
                part_index: self.part_index,
                provider_state: Some(
                    ReplayState::FunctionCall(FunctionCallState {
                        id: self.item_id.clone(),
                        call_id: self.call_id.clone(),
                        name: self.name.clone(),
                        arguments: if arguments.is_empty() {
                            self.arguments.clone()
                        } else {
                            arguments.to_owned()
                        },
                    })
                    .encode(&self.model_id)?,
                ),
            });
        }
        Ok(())
    }
}
