//! Per-part Gemini replay retains original signed boundaries and whitespace.

use ai_interface::{ModelEvent, ProviderState};

use super::super::state::{
    GoogleAssistantPart, GoogleFunctionCallContext, GooglePartState, GoogleStateScope,
};
use super::{part::AccumulatedPart, types::GoogleStreamError};

#[derive(Debug, Default)]
pub(super) struct ReplayParts {
    parts: Vec<ReplayPart>,
}

#[derive(Debug)]
struct ReplayPart {
    original: GoogleAssistantPart,
    owner: Option<usize>,
}

impl ReplayParts {
    pub(super) fn append(
        &mut self,
        part: GoogleAssistantPart,
        merge_with_previous: bool,
        owner: Option<usize>,
    ) {
        if let Some(owner) = owner {
            for pending in self
                .parts
                .iter_mut()
                .rev()
                .take_while(|part| part.owner.is_none())
            {
                pending.owner = Some(owner);
            }
        }
        if merge_with_previous
            && part.thought_signature.is_none()
            && part.function_call.is_none()
            && let Some(text) = &part.text
            && let Some(previous) = self.parts.last_mut()
            && previous.original.thought_signature.is_none()
            && previous.original.function_call.is_none()
            && previous.original.thought == part.thought
            && let Some(previous_text) = &mut previous.original.text
        {
            previous_text.push_str(text);
            return;
        }
        self.parts.push(ReplayPart {
            original: part,
            owner,
        });
    }

    pub(super) fn tool_state(
        &self,
        part_index: usize,
        model: &str,
        unsigned_call: GoogleFunctionCallContext,
    ) -> Result<ProviderState, GoogleStreamError> {
        let mut state = self.part_state(part_index);
        state.unsigned_call = Some(unsigned_call);
        match state.encode(model) {
            Ok(state) => Ok(state),
            Err(source) => Err(GoogleStreamError::SerializeState { source }),
        }
    }

    pub(super) fn state_events(
        &mut self,
        visible: &[AccumulatedPart],
        next_part_index: &mut usize,
        model: &str,
    ) -> Result<Vec<ModelEvent>, GoogleStreamError> {
        if !self
            .parts
            .iter()
            .any(|part| part.original.thought_signature.is_some())
        {
            return self.unsigned_state_events(visible, model);
        }
        let mut targets = visible
            .iter()
            .filter_map(|part| part.part_index.map(|index| (index, part.is_reasoning())))
            .collect::<Vec<_>>();
        if self.parts.last().is_some_and(|part| part.owner.is_none()) {
            let previous = self.parts.iter().rev().find_map(|part| part.owner);
            let owner =
                match previous.filter(|index| targets.iter().any(|(target, _)| index == target)) {
                    Some(index) => index,
                    None => {
                        let index = *next_part_index;
                        *next_part_index = next_part_index.saturating_add(1);
                        targets.push((index, true));
                        index
                    }
                };
            for pending in self
                .parts
                .iter_mut()
                .rev()
                .take_while(|part| part.owner.is_none())
            {
                pending.owner = Some(owner);
            }
        }
        let mut events = Vec::new();
        for (part_index, reasoning) in targets {
            let state = match self.part_state(part_index).encode(model) {
                Ok(state) => state,
                Err(source) => return Err(GoogleStreamError::SerializeState { source }),
            };
            events.push(if reasoning {
                ModelEvent::ReasoningDelta {
                    part_index,
                    delta: String::new(),
                    provider_state: Some(state),
                }
            } else {
                ModelEvent::TextDelta {
                    part_index,
                    delta: String::new(),
                    provider_state: Some(state),
                }
            });
        }
        Ok(events)
    }

    fn unsigned_state_events(
        &self,
        visible: &[AccumulatedPart],
        model: &str,
    ) -> Result<Vec<ModelEvent>, GoogleStreamError> {
        let mut text_indices = visible
            .iter()
            .filter_map(|part| part.assistant_text().and(part.part_index));
        let Some(first_text) = text_indices.next() else {
            return Ok(Vec::new());
        };
        let call_before_text = self.parts.iter().any(|part| {
            part.original.function_call.is_some()
                && part.owner.is_some_and(|index| index < first_text)
        });
        if text_indices.next().is_none() && !call_before_text {
            return Ok(Vec::new());
        }
        let mut state = GooglePartState::new(Vec::new());
        state.scope = GoogleStateScope::NormalizedText;
        let state = match state.encode(model) {
            Ok(state) => state,
            Err(source) => return Err(GoogleStreamError::SerializeState { source }),
        };
        Ok(vec![ModelEvent::TextDelta {
            part_index: first_text,
            delta: String::new(),
            provider_state: Some(state),
        }])
    }

    fn part_state(&self, part_index: usize) -> GooglePartState {
        GooglePartState::new(
            self.parts
                .iter()
                .filter(|part| part.owner == Some(part_index))
                .map(|part| part.original.clone())
                .collect(),
        )
    }
}
