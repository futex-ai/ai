//! Part positions and ordered events, including delayed tool identities.

use std::collections::{BTreeMap, VecDeque, btree_map::Entry};

use ai_interface::{ConversationPart, ModelEvent, ModelResponse};
use ai_models_core::synthetic_tool_call_id;

use crate::{
    ChatCompletionsToolFunction,
    chat_completions::{ChatCompletionsStreamChunk, ChatCompletionsToolCallDelta},
};

use super::{
    dialect::CompletionContext,
    stream_tools::{ToolPart, remove_part},
};

#[derive(Default)]
pub(super) struct EventParts {
    next_index: usize,
    text: Option<usize>,
    reasoning: Option<usize>,
    tools: BTreeMap<u64, ToolPart>,
    legacy: Option<LegacyPart>,
    pending: VecDeque<PendingEvent>,
}

enum PendingEvent {
    Content(ModelEvent),
    ToolStart(u64),
    LegacyStart,
}

struct LegacyPart {
    part_index: usize,
    identity: Option<(String, String)>,
}

impl EventParts {
    pub(super) fn push_chunk(
        &mut self,
        chunk: &ChatCompletionsStreamChunk,
        include_legacy: bool,
    ) -> VecDeque<ModelEvent> {
        for choice in &chunk.choices {
            if choice.index != 0 {
                continue;
            }
            if let Some(delta) = nonempty(choice.delta.reasoning_content.as_deref()) {
                let part_index = *self
                    .reasoning
                    .get_or_insert_with(|| take_index(&mut self.next_index));
                self.pending
                    .push_back(PendingEvent::Content(ModelEvent::ReasoningDelta {
                        part_index,
                        delta,
                        provider_state: None,
                    }));
            }
            if let Some(delta) = nonempty(choice.delta.content.as_deref()) {
                let part_index = *self
                    .text
                    .get_or_insert_with(|| take_index(&mut self.next_index));
                self.pending
                    .push_back(PendingEvent::Content(ModelEvent::TextDelta {
                        part_index,
                        delta,
                        provider_state: None,
                    }));
            }
            for call in &choice.delta.tool_calls {
                self.discard_legacy();
                self.push_tool(call);
            }
            if include_legacy
                && self.tools.is_empty()
                && let Some(function) = choice.delta.function_call.as_ref()
            {
                let legacy = self.legacy.get_or_insert_with(|| {
                    self.pending.push_back(PendingEvent::LegacyStart);
                    LegacyPart {
                        part_index: take_index(&mut self.next_index),
                        identity: None,
                    }
                });
                if let Some(delta) = nonempty(function.arguments.as_deref()) {
                    self.pending.push_back(PendingEvent::Content(
                        ModelEvent::ToolCallArgumentsDelta {
                            part_index: legacy.part_index,
                            delta,
                        },
                    ));
                }
            }
        }
        self.ready_events()
    }

    pub(super) fn terminal_text(&mut self, delta: String) -> ModelEvent {
        let part_index = *self
            .text
            .get_or_insert_with(|| take_index(&mut self.next_index));
        ModelEvent::TextDelta {
            part_index,
            delta,
            provider_state: None,
        }
    }

    pub(super) fn finish(
        &mut self,
        context: CompletionContext<'_>,
        function: Option<ChatCompletionsToolFunction>,
        response: &ModelResponse,
    ) -> VecDeque<ModelEvent> {
        if let (Some(legacy), Some(function)) = (self.legacy.as_mut(), function) {
            let id = synthetic_tool_call_id(
                "xai_legacy_function_call:",
                context.synthetic_tool_call_scope,
                0,
                &function.name,
                &function.arguments,
            );
            legacy.identity = Some((id, function.name));
        }
        let mut indices = self
            .tools
            .values()
            .map(|tool| (tool.part_index(), tool.id()))
            .collect::<Vec<_>>();
        if let Some(legacy) = self.legacy.as_ref() {
            indices.push((
                legacy.part_index,
                legacy.identity.as_ref().map(|(id, _)| id.as_str()),
            ));
        }
        indices.sort_by_key(|(index, _)| *index);
        self.pending
            .extend(indices.into_iter().map(|(part_index, id)| {
                PendingEvent::Content(ModelEvent::ToolCallEnd {
                    part_index,
                    provider_state: response.parts.iter().find_map(|part| match part {
                        ConversationPart::ToolCall {
                            call_id,
                            provider_state,
                            ..
                        } if Some(call_id.as_str()) == id => provider_state.clone(),
                        _ => None,
                    }),
                })
            }));
        let mut events = self.ready_events();
        for part in &response.parts {
            let Some(state) = part.provider_state().cloned() else {
                continue;
            };
            let event = match part {
                ConversationPart::Text { .. } => ModelEvent::TextDelta {
                    part_index: *self
                        .text
                        .get_or_insert_with(|| take_index(&mut self.next_index)),
                    delta: String::new(),
                    provider_state: Some(state),
                },
                ConversationPart::Reasoning { .. } => ModelEvent::ReasoningDelta {
                    part_index: *self
                        .reasoning
                        .get_or_insert_with(|| take_index(&mut self.next_index)),
                    delta: String::new(),
                    provider_state: Some(state),
                },
                _ => continue,
            };
            events.push_back(event);
        }
        events
    }

    fn push_tool(&mut self, call: &ChatCompletionsToolCallDelta) {
        let tool = match self.tools.entry(call.index) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                self.pending.push_back(PendingEvent::ToolStart(call.index));
                entry.insert(ToolPart::new(take_index(&mut self.next_index)))
            }
        };
        if let Some(event) = tool.push(call) {
            self.pending.push_back(PendingEvent::Content(event));
        }
    }

    fn ready_events(&mut self) -> VecDeque<ModelEvent> {
        let mut events = VecDeque::new();
        while let Some(pending) = self.pending.front() {
            let start = match pending {
                PendingEvent::Content(_) => None,
                PendingEvent::ToolStart(index) => self.tools.get(index).and_then(ToolPart::start),
                PendingEvent::LegacyStart => self.legacy.as_ref().and_then(|legacy| {
                    let (id, name) = legacy.identity.as_ref()?;
                    Some(ModelEvent::ToolCallStart {
                        part_index: legacy.part_index,
                        id: id.clone(),
                        name: name.clone(),
                        operation_id: Some(id.clone()),
                    })
                }),
            };
            if !matches!(pending, PendingEvent::Content(_)) && start.is_none() {
                break;
            }
            if let Some(PendingEvent::Content(event)) = self.pending.pop_front() {
                events.push_back(event);
            } else if let Some(start) = start {
                events.push_back(start);
            }
        }
        events
    }

    fn discard_legacy(&mut self) {
        let Some(legacy) = self.legacy.take() else {
            return;
        };
        self.pending.retain_mut(|pending| match pending {
            PendingEvent::LegacyStart => false,
            PendingEvent::Content(event) => !remove_part(event, legacy.part_index),
            PendingEvent::ToolStart(_) => true,
        });
        for tool in self.tools.values_mut() {
            tool.remove_prior_part(legacy.part_index);
        }
        for index in [&mut self.text, &mut self.reasoning].into_iter().flatten() {
            if *index > legacy.part_index {
                *index -= 1;
            }
        }
        self.next_index -= 1;
    }
}

pub(super) fn terminal_events(response: ModelResponse) -> [ModelEvent; 2] {
    [
        ModelEvent::Usage {
            usage: response.usage,
        },
        ModelEvent::Finish {
            finish_reason: response.finish_reason,
            provider: response.provider,
            model_id: response.model_id,
            catalog_model_id: response.catalog_model_id,
            thinking_level: response.thinking_level,
            structured_output: response.structured_output,
        },
    ]
}

fn take_index(next_index: &mut usize) -> usize {
    let index = *next_index;
    *next_index += 1;
    index
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value.filter(|text| !text.is_empty()).map(str::to_owned)
}
