//! Ordered Gemini replay parts retain signed provider boundaries.

use ai_interface::{GoogleAssistantPart, ProviderConversationItem};

#[derive(Debug, Default)]
pub(super) struct ReplayParts {
    parts: Vec<GoogleAssistantPart>,
}

impl ReplayParts {
    pub(super) fn append(&mut self, part: GoogleAssistantPart, merge_with_previous: bool) {
        if merge_with_previous
            && part.thought_signature.is_none()
            && part.function_call.is_none()
            && let Some(text) = &part.text
            && let Some(previous) = self.parts.last_mut()
            && previous.thought_signature.is_none()
            && previous.function_call.is_none()
            && previous.thought == part.thought
            && let Some(previous_text) = &mut previous.text
        {
            previous_text.push_str(text);
            return;
        }
        self.parts.push(part);
    }

    pub(super) fn provider_context(&self) -> Vec<ProviderConversationItem> {
        if self
            .parts
            .iter()
            .any(|part| part.thought_signature.is_some())
        {
            vec![ProviderConversationItem::GoogleAssistantMessage {
                parts: self.parts.clone(),
            }]
        } else {
            Vec::new()
        }
    }
}
