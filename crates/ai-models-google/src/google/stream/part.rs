//! Visible Gemini content accumulation without replay signature boundaries.

use ai_interface::GoogleAssistantPart;

use super::types::GoogleStreamDelta;

#[derive(Debug)]
pub(super) struct AccumulatedPart {
    part: GoogleAssistantPart,
    assistant_emitted: bool,
    pub(super) part_index: Option<usize>,
}

impl AccumulatedPart {
    pub(super) fn append(
        parts: &mut Vec<Self>,
        mut part: GoogleAssistantPart,
        merge_with_previous: bool,
    ) -> Option<(usize, Option<GoogleStreamDelta>)> {
        if part.thought_signature.is_some() && part.text.is_none() && part.function_call.is_none() {
            return None;
        }
        part.thought_signature = None;
        let previous_index = parts.len().saturating_sub(1);
        if merge_with_previous
            && part.function_call.is_none()
            && let Some(text) = &part.text
            && let Some(previous) = parts.last_mut()
            && previous.part.function_call.is_none()
            && previous.part.text.is_some()
            && previous.part.thought == part.thought
        {
            return Some((previous_index, previous.append_text(text)));
        }
        let delta = part
            .text
            .as_ref()
            .and_then(|text| initial_delta(text, part.thought));
        let assistant_emitted = matches!(
            delta,
            Some(GoogleStreamDelta::AssistantText {
                starts_part: true,
                ..
            })
        );
        parts.push(Self {
            part,
            assistant_emitted,
            part_index: None,
        });
        Some((parts.len() - 1, delta))
    }

    fn append_text(&mut self, delta: &str) -> Option<GoogleStreamDelta> {
        let text = self.part.text.get_or_insert_default();
        text.push_str(delta);
        if delta.is_empty() {
            return None;
        }
        if self.part.thought == Some(true) {
            return Some(GoogleStreamDelta::ReasoningText {
                delta: delta.to_owned(),
            });
        }
        if self.assistant_emitted {
            return Some(GoogleStreamDelta::AssistantText {
                delta: delta.to_owned(),
                starts_part: false,
            });
        }
        if text.trim().is_empty() {
            return None;
        }
        self.assistant_emitted = true;
        Some(GoogleStreamDelta::AssistantText {
            delta: text.clone(),
            starts_part: true,
        })
    }

    pub(super) fn assistant_text(&self) -> Option<&str> {
        self.part
            .text
            .as_deref()
            .filter(|text| self.part.thought != Some(true) && !text.trim().is_empty())
    }
}

fn initial_delta(text: &str, thought: Option<bool>) -> Option<GoogleStreamDelta> {
    if text.is_empty() {
        return None;
    }
    if thought == Some(true) {
        return Some(GoogleStreamDelta::ReasoningText {
            delta: text.to_owned(),
        });
    }
    if text.trim().is_empty() {
        return None;
    }
    Some(GoogleStreamDelta::AssistantText {
        delta: text.to_owned(),
        starts_part: true,
    })
}
