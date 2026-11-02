//! Composition of installed provider converters for legacy stored messages.

use super::{
    conversation::{ConversationMessage, ConversationRole},
    legacy::{
        LegacyConversationMessage, LegacyPartAttachment, LegacyPartOrder, LegacyPartTarget,
        LegacyProviderConverter,
    },
    legacy_fallback::preserve_item,
    part::ConversationPart,
};

/// Converts a version one message without exposing provider types in contracts.
pub fn compose_legacy_message(
    message: &LegacyConversationMessage,
    converters: &[&dyn LegacyProviderConverter],
) -> ConversationMessage {
    if message.role != ConversationRole::User && !message.content_parts.is_empty() {
        tracing::warn!("dropping legacy content_parts from an assistant or tool message");
    }
    let mut result = portable_message(message);
    if message.role != ConversationRole::Assistant {
        return result;
    }
    let primary = message.provider_context.first().and_then(|item| {
        converters
            .iter()
            .find(|converter| converter.convert_item(item, message, 0).is_some())
            .copied()
    });
    let order = primary
        .map(|converter| converter.order())
        .unwrap_or_default();
    let override_parts = primary.and_then(|converter| converter.convert_message(message));
    let has_override = override_parts.is_some();
    if let Some(parts) = override_parts {
        result.parts = parts;
    }
    let mut positions = Vec::new();
    for (item_index, item) in message.provider_context.iter().enumerate() {
        let converted = converters.iter().find_map(|converter| {
            converter
                .convert_item(item, message, item_index)
                .map(|parts| (*converter, parts))
        });
        let (attachments, installed) = match converted {
            Some((converter, _))
                if has_override
                    && primary
                        .is_some_and(|primary| primary.provider() == converter.provider()) =>
            {
                continue;
            }
            Some((_, parts)) => (parts, true),
            None => (vec![preserve_item(item)], false),
        };
        for attachment in attachments {
            if let Some(position) = attach(&mut result.parts, attachment, installed) {
                if !positions.contains(&position) {
                    positions.push(position);
                }
            } else {
                tracing::warn!(
                    item_type = item.kind,
                    "dropping legacy provider item without a matching part"
                );
            }
        }
    }
    if !has_override {
        result.parts = order_parts(result.parts, positions, order);
    }
    result
}

fn portable_message(message: &LegacyConversationMessage) -> ConversationMessage {
    let parts = match message.role {
        ConversationRole::User if !message.content_parts.is_empty() => {
            message.content_parts.clone()
        }
        ConversationRole::User | ConversationRole::Assistant => {
            let mut parts = Vec::new();
            if !message.content.is_empty() {
                parts.push(ConversationPart::text(&message.content));
            }
            if message.role == ConversationRole::Assistant {
                parts.extend(
                    message
                        .tool_calls
                        .iter()
                        .cloned()
                        .map(ConversationPart::from_tool_call),
                );
            }
            parts
        }
        ConversationRole::Tool => vec![ConversationPart::tool_result(
            &message.content,
            message.name.as_deref().unwrap_or_default(),
            message.tool_call_id.as_deref().unwrap_or_default(),
        )],
    };
    ConversationMessage {
        role: message.role,
        parts,
        name: (message.role != ConversationRole::Tool)
            .then(|| message.name.clone())
            .flatten(),
    }
}

fn attach(
    parts: &mut Vec<ConversationPart>,
    attachment: LegacyPartAttachment,
    installed: bool,
) -> Option<usize> {
    let target = match &attachment.target {
        LegacyPartTarget::Text => parts.iter().position(|part| matches!(part, ConversationPart::Text { .. })),
        LegacyPartTarget::ToolCall { call_id } => parts.iter().position(|part| matches!(part, ConversationPart::ToolCall { call_id: existing, .. } if existing == call_id)),
        LegacyPartTarget::Reasoning { text } => {
            parts.push(ConversationPart::reasoning(text).with_provider_state(attachment.provider_state));
            return Some(parts.len() - 1);
        }
    };
    if let Some(index) = target {
        if parts[index].provider_state().is_none() {
            *parts[index].provider_state_mut() = Some(attachment.provider_state);
            return Some(index);
        }
    } else if installed {
        return None;
    }
    parts.push(ConversationPart::reasoning("").with_provider_state(attachment.provider_state));
    Some(parts.len() - 1)
}

fn order_parts(
    parts: Vec<ConversationPart>,
    mut positions: Vec<usize>,
    order: LegacyPartOrder,
) -> Vec<ConversationPart> {
    if order == LegacyPartOrder::ReasoningFirst {
        let (mut reasoning, rest): (Vec<_>, Vec<_>) = parts
            .into_iter()
            .partition(|part| matches!(part, ConversationPart::Reasoning { .. }));
        reasoning.extend(rest);
        return reasoning;
    }
    for position in 0..parts.len() {
        if !positions.contains(&position) {
            positions.push(position);
        }
    }
    let mut parts = parts.into_iter().map(Some).collect::<Vec<_>>();
    positions
        .into_iter()
        .filter_map(|index| parts.get_mut(index).and_then(Option::take))
        .collect()
}
