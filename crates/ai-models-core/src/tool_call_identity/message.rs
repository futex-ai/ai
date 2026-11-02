//! Portable history keeps its established ids while new parts extend the hash.

use ai_interface::{ConversationMessage, ConversationPart, ConversationRole};

use super::hasher::StableHasher;

impl StableHasher {
    pub(super) fn write_message(&mut self, message: &ConversationMessage) {
        self.write_role(message.role);
        self.write_str(&message.text());
        let media = message.parts.iter().any(|part| {
            matches!(
                part,
                ConversationPart::Image { .. } | ConversationPart::Video { .. }
            )
        });
        self.write_usize(if media { message.parts.len() } else { 0 });
        if media {
            for part in &message.parts {
                self.write_part(part);
            }
        }
        let result = message.parts.iter().find_map(|part| match part {
            ConversationPart::ToolResult {
                call_id, tool_name, ..
            } => Some((call_id.as_str(), tool_name.as_str())),
            _ => None,
        });
        self.write_optional_str(result.map(|(_, name)| name).or(message.name.as_deref()));
        self.write_optional_str(result.map(|(call_id, _)| call_id));
        let calls = message.tool_calls();
        self.write_usize(calls.len());
        for call in calls {
            self.write_str(&call.id);
            self.write_str(&call.name);
            self.write_json(&call.input);
            self.write_optional_str(call.operation_id.as_deref());
        }
        self.write_usize(
            message
                .parts
                .iter()
                .filter(|part| part.provider_state().is_some())
                .count(),
        );
        for (index, part) in message.parts.iter().enumerate() {
            if let Some(state) = part.provider_state() {
                self.write_usize(index);
                self.write_provider_state(Some(state));
            }
        }
        if !legacy_layout(message, media) {
            self.write_str("ordered_parts");
            self.write_usize(message.parts.len());
            for part in &message.parts {
                self.write_part(part);
            }
        }
    }
}

fn legacy_layout(message: &ConversationMessage, media: bool) -> bool {
    match message.role {
        ConversationRole::User => {
            message.parts.iter().all(|part| {
                matches!(
                    part,
                    ConversationPart::Text { .. }
                        | ConversationPart::Image { .. }
                        | ConversationPart::Video { .. }
                )
            }) && (media || message.parts.len() <= 1)
        }
        ConversationRole::Assistant => message.parts.iter().enumerate().all(|(index, part)| {
            matches!(part, ConversationPart::ToolCall { .. })
                || index == 0 && matches!(part, ConversationPart::Text { .. })
        }),
        ConversationRole::Tool => matches!(
            message.parts.as_slice(),
            [ConversationPart::ToolResult { .. }]
        ),
    }
}
