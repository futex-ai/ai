//! Google typed fragment state and direct indexed event mapping.

use ai_interface::ModelEvent;
use ai_models_core::synthetic_tool_call_id;
use serde_json::{Map, Value};

use super::{
    part::AccumulatedPart,
    replay::ReplayParts,
    types::{
        GoogleCompletion, GooglePromptFeedback, GoogleStreamChunk, GoogleStreamDelta,
        GoogleStreamError, GoogleStreamUpdate, GoogleUsageMetadata,
    },
};

#[derive(Debug, Default)]
pub(super) struct GoogleStreamAccumulator {
    candidate_seen: bool,
    visible_parts: Vec<AccumulatedPart>,
    replay_parts: ReplayParts,
    finish_reason: Option<String>,
    prompt_feedback: Option<GooglePromptFeedback>,
    usage: Option<GoogleUsageMetadata>,
    assistant_parts_emitted: usize,
    next_part_index: usize,
    tool_call_count: usize,
    synthetic_tool_call_scope: String,
}

impl GoogleStreamAccumulator {
    pub(super) fn new(synthetic_tool_call_scope: String) -> Self {
        Self {
            synthetic_tool_call_scope,
            ..Self::default()
        }
    }

    pub(super) fn push_data(
        &mut self,
        data: &str,
    ) -> Result<GoogleStreamUpdate, GoogleStreamError> {
        let chunk = match serde_json::from_str::<GoogleStreamChunk>(data) {
            Ok(chunk) => chunk,
            Err(source) => return Err(GoogleStreamError::DeserializeChunk { source }),
        };
        if let Some(error) = chunk.error {
            return Ok(GoogleStreamUpdate::ProviderError(error));
        }
        if let Some(prompt_feedback) = chunk.prompt_feedback {
            self.prompt_feedback = Some(prompt_feedback);
        }
        if let Some(usage) = chunk.usage_metadata {
            self.usage = Some(usage);
        }
        let mut events = Vec::new();
        if let Some(candidate) = chunk.candidates.into_iter().next() {
            self.candidate_seen = true;
            if let Some(content) = candidate.content {
                for (index, part) in content.parts.into_iter().enumerate() {
                    self.replay_parts.append(part.clone(), index == 0);
                    let call = part.function_call.clone();
                    if let Some((slot, Some(delta))) =
                        AccumulatedPart::append(&mut self.visible_parts, part, index == 0)
                    {
                        events.push(self.text_event(slot, delta));
                    }
                    if let Some(call) = call {
                        let input = call.args.unwrap_or_else(|| Value::Object(Map::new()));
                        let arguments = input.to_string();
                        let (id, operation_id) = match call.id {
                            Some(id) => (id, None),
                            None => {
                                let id = synthetic_tool_call_id(
                                    "google_function_call:",
                                    &self.synthetic_tool_call_scope,
                                    self.tool_call_count,
                                    &call.name,
                                    &arguments,
                                );
                                (id.clone(), Some(id))
                            }
                        };
                        let part_index = self.next_part_index;
                        self.next_part_index = self.next_part_index.saturating_add(1);
                        self.tool_call_count = self.tool_call_count.saturating_add(1);
                        events.extend([
                            ModelEvent::ToolCallStart {
                                part_index,
                                id,
                                name: call.name,
                                operation_id,
                            },
                            ModelEvent::ToolCallArgumentsDelta {
                                part_index,
                                delta: arguments,
                            },
                            ModelEvent::ToolCallEnd { part_index },
                        ]);
                    }
                }
            }
            if let Some(finish_reason) = candidate.finish_reason {
                self.finish_reason = Some(finish_reason);
            }
        }
        Ok(GoogleStreamUpdate::Continue { events })
    }

    fn text_event(&mut self, slot: usize, delta: GoogleStreamDelta) -> ModelEvent {
        let part = &mut self.visible_parts[slot];
        let part_index = match part.part_index {
            Some(index) => index,
            None => {
                let index = self.next_part_index;
                self.next_part_index = self.next_part_index.saturating_add(1);
                part.part_index = Some(index);
                index
            }
        };
        match delta {
            GoogleStreamDelta::AssistantText {
                mut delta,
                starts_part,
            } => {
                if starts_part {
                    if self.assistant_parts_emitted > 0 {
                        delta.insert(0, '\n');
                    }
                    self.assistant_parts_emitted = self.assistant_parts_emitted.saturating_add(1);
                }
                ModelEvent::TextDelta { part_index, delta }
            }
            GoogleStreamDelta::ReasoningText { delta } => {
                ModelEvent::ReasoningDelta { part_index, delta }
            }
        }
    }

    pub(super) fn finish(&self) -> Result<GoogleCompletion, GoogleStreamError> {
        let prompt_blocked = self
            .prompt_feedback
            .as_ref()
            .and_then(|feedback| feedback.block_reason.as_ref())
            .is_some();
        if self.finish_reason.is_none() && !prompt_blocked {
            return Err(GoogleStreamError::MissingTerminal);
        }
        Ok(GoogleCompletion {
            candidate_seen: self.candidate_seen,
            assistant_message: self
                .visible_parts
                .iter()
                .filter_map(AccumulatedPart::assistant_text)
                .collect::<Vec<_>>()
                .join("\n"),
            tool_call_count: self.tool_call_count,
            finish_reason: self.finish_reason.clone(),
            usage: self.usage.clone().unwrap_or_default(),
            provider_context: self.replay_parts.provider_context(),
        })
    }
}
