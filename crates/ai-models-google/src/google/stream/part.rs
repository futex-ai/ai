//! Gemini content parts and unsigned text-fragment accumulation.

use ai_interface::GoogleAssistantPart;
use serde_json::{Value, json};

use super::types::GoogleStreamDelta;

#[derive(Debug)]
pub(super) struct AccumulatedPart {
    part: GoogleAssistantPart,
    assistant_emitted: bool,
}

impl AccumulatedPart {
    pub(super) fn append(
        parts: &mut Vec<Self>,
        part: GoogleAssistantPart,
        merge_with_previous: bool,
    ) -> Option<GoogleStreamDelta> {
        if merge_with_previous
            && part.thought_signature.is_none()
            && part.function_call.is_none()
            && let Some(text) = &part.text
            && let Some(previous) = parts.last_mut()
            && previous.part.thought_signature.is_none()
            && previous.part.function_call.is_none()
            && previous.part.text.is_some()
            && previous.part.thought == part.thought
        {
            return previous.append_text(text);
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
        });
        delta
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

    pub(super) fn body(&self) -> Value {
        json!(&self.part)
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
