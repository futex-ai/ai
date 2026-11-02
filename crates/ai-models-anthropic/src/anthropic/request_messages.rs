//! Ordered conversation parts and private Anthropic continuation mapping.

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ModelError, ModelResult,
};

use super::{
    replay::{AnthropicAssistantBlock, AnthropicReplay, ReplayMode},
    request_types::{AnthropicBlock, AnthropicImageSource, AnthropicMessage},
};

const PROVIDER: &str = "anthropic";

pub(super) fn anthropic_messages(
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<AnthropicMessage>> {
    let mut output = Vec::new();
    for message in messages {
        message.validate(PROVIDER, model_id)?;
        let replay = message
            .parts
            .iter()
            .map(|part| match part.provider_state() {
                Some(state) => AnthropicReplay::decode(state, model_id),
                None => Ok(None),
            })
            .collect::<ModelResult<Vec<_>>>()?;
        let has_signed_blocks = message.parts.iter().zip(&replay).any(|(part, state)| {
            state.as_ref().is_some_and(|state| {
                matches!(part, ConversationPart::Reasoning { .. })
                    || state.blocks.iter().any(|block| block.is_reasoning())
            })
        });
        let has_normalized_text = replay
            .iter()
            .flatten()
            .any(|state| state.mode == ReplayMode::NormalizedText);
        let has_original_text = replay.iter().flatten().any(|state| {
            state.mode == ReplayMode::Original
                && state
                    .blocks
                    .iter()
                    .any(|block| matches!(block, AnthropicAssistantBlock::Text { .. }))
        });
        let preserve_empty_assistant = message.role == ConversationRole::Assistant
            && replay
                .iter()
                .flatten()
                .any(|state| state.mode == ReplayMode::Original);
        let blocks = if message.role == ConversationRole::Assistant
            && has_normalized_text
            && !has_signed_blocks
            && !has_original_text
        {
            normalized_assistant(message, replay)
        } else {
            ordered_blocks(model_id, message, replay)?
        };
        let role = match message.role {
            ConversationRole::Assistant => "assistant",
            ConversationRole::Tool | ConversationRole::User => "user",
        };
        if blocks.is_empty() && preserve_empty_assistant {
            output.push(AnthropicMessage {
                role: role.to_owned(),
                content: blocks,
            });
        } else {
            append_blocks(&mut output, role, blocks);
        }
    }
    Ok(output)
}

fn normalized_assistant(
    message: &ConversationMessage,
    replay: Vec<Option<AnthropicReplay>>,
) -> Vec<AnthropicBlock> {
    let mut blocks = Vec::new();
    let text = message.text();
    if !text.is_empty() {
        blocks.push(AnthropicBlock::Text { text });
    }
    for (part, own_state) in message.parts.iter().zip(replay) {
        if let Some(state) = own_state {
            blocks.extend(
                state
                    .blocks
                    .into_iter()
                    .filter(|block| !matches!(block, AnthropicAssistantBlock::Text { .. }))
                    .map(AnthropicBlock::from),
            );
        } else if let ConversationPart::ToolCall {
            call_id,
            tool_name,
            arguments,
            ..
        } = part
        {
            blocks.push(AnthropicBlock::ToolUse {
                id: call_id.clone(),
                name: tool_name.clone(),
                input: arguments.clone(),
            });
        }
    }
    blocks
}

fn ordered_blocks(
    model_id: &str,
    message: &ConversationMessage,
    replay: Vec<Option<AnthropicReplay>>,
) -> ModelResult<Vec<AnthropicBlock>> {
    let mut blocks = Vec::new();
    for (part, own_state) in message.parts.iter().zip(replay) {
        if let Some(state) = own_state {
            blocks.extend(state.blocks.into_iter().map(AnthropicBlock::from));
            continue;
        }
        match part {
            ConversationPart::Text { text, .. } => {
                blocks.push(AnthropicBlock::Text { text: text.clone() });
            }
            ConversationPart::Image {
                mime_type,
                data_base64,
                ..
            } => {
                blocks.push(AnthropicBlock::Image {
                    source: AnthropicImageSource {
                        kind: "base64".to_owned(),
                        media_type: mime_type.clone(),
                        data: data_base64.clone(),
                    },
                });
            }
            ConversationPart::Video { .. } => {
                return Err(ModelError::provider(
                    PROVIDER,
                    model_id,
                    "Anthropic accepts text and image content parts only",
                ));
            }
            ConversationPart::ToolCall {
                call_id,
                tool_name,
                arguments,
                ..
            } => {
                blocks.push(AnthropicBlock::ToolUse {
                    id: call_id.clone(),
                    name: tool_name.clone(),
                    input: arguments.clone(),
                });
            }
            ConversationPart::ToolResult {
                call_id, content, ..
            } => {
                blocks.push(AnthropicBlock::ToolResult {
                    tool_use_id: Some(call_id.clone()),
                    content: content.clone(),
                });
            }
            ConversationPart::Reasoning { .. } => {}
        }
    }
    if message.role == ConversationRole::User && blocks.is_empty() {
        blocks.push(AnthropicBlock::Text {
            text: String::new(),
        });
    }
    Ok(blocks)
}

fn append_blocks(messages: &mut Vec<AnthropicMessage>, role: &str, blocks: Vec<AnthropicBlock>) {
    if blocks.is_empty() {
        return;
    }
    if let Some(existing) = messages.last_mut()
        && existing.role == role
        && role == "user"
    {
        existing.content.extend(blocks);
        return;
    }
    messages.push(AnthropicMessage {
        role: role.to_owned(),
        content: blocks,
    });
}
