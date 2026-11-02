//! Decode own state and keep legacy snapshots separate from native part replay.

use ai_interface::{ConversationMessage, ConversationPart, ModelResult};

use crate::minimax::state::ReasoningDetail;

use super::{
    dialect::{Dialect, ReplayRule},
    replay_state::{self, DecodedState, PartState, RawToolCall},
    request_legacy::LegacyFunctionCall,
    request_types::{ChatCompletionsContent, ChatCompletionsMessage},
    tools::raw_tool_call,
};

#[derive(Default)]
pub(super) struct MessageReplay {
    pub(super) snapshot: Option<ChatCompletionsMessage>,
    pub(super) reasoning_snapshot: Option<ReasoningSnapshot>,
    pub(super) text: Option<Option<String>>,
    pub(super) reasoning_content: Option<String>,
    pub(super) reasoning_details: Vec<ReasoningDetail>,
    pub(super) calls: Vec<(String, RawToolCall)>,
    pub(super) legacy: Option<LegacyFunctionCall>,
}

/// The first version one MiniMax item, separate from native part accumulation.
pub(super) struct ReasoningSnapshot {
    /// Whole-message reasoning from the first item.
    pub(super) content: Option<String>,
    /// Whole-message details from the same item.
    pub(super) details: Vec<ReasoningDetail>,
}

pub(super) fn decode_message(
    dialect: &Dialect,
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<MessageReplay> {
    let mut replay = MessageReplay::default();
    let mut has_text_state = false;
    for part in &message.parts {
        let state = match part.provider_state() {
            Some(state) => replay_state::decode(dialect, model_id, state)?,
            None => None,
        };
        if let ConversationPart::Text { text, .. } = part
            && !matches!(state, Some(DecodedState::Part(PartState::Text { .. })))
        {
            replay
                .text
                .get_or_insert(None)
                .get_or_insert_with(String::new)
                .push_str(text);
        }
        let Some(state) = state else { continue };
        match state {
            DecodedState::Assistant {
                content,
                reasoning_content,
                tool_calls,
            } => {
                if replay.snapshot.is_none() {
                    let content = match dialect.replay {
                        ReplayRule::Qwen => {
                            content.or_else(|| tool_calls.is_empty().then(String::new))
                        }
                        _ => content,
                    };
                    replay.snapshot = Some(ChatCompletionsMessage {
                        role: "assistant",
                        content: Some(content.map(ChatCompletionsContent::Text)),
                        name: None,
                        tool_call_id: None,
                        reasoning_content,
                        reasoning_details: Vec::new(),
                        tool_calls: tool_calls
                            .iter()
                            .map(|call| raw_tool_call(&call.id, &call.name, &call.arguments))
                            .collect(),
                        function_call: None,
                    });
                }
            }
            DecodedState::Part(PartState::Text { content }) => {
                if matches!(part, ConversationPart::Text { .. }) {
                    has_text_state = true;
                    let text = replay.text.get_or_insert(None);
                    if let Some(content) = content {
                        text.get_or_insert_with(String::new).push_str(&content);
                    }
                }
            }
            DecodedState::Part(PartState::Reasoning {
                reasoning_content,
                reasoning_details,
            }) => {
                if matches!(part, ConversationPart::Reasoning { .. }) {
                    if let Some(reasoning) = reasoning_content {
                        replay
                            .reasoning_content
                            .get_or_insert_with(String::new)
                            .push_str(&reasoning);
                    }
                    replay.reasoning_details.extend(reasoning_details);
                }
            }
            DecodedState::Reasoning {
                reasoning_content,
                reasoning_details,
            } => {
                if matches!(part, ConversationPart::Reasoning { .. })
                    && replay.reasoning_snapshot.is_none()
                {
                    replay.reasoning_snapshot = Some(ReasoningSnapshot {
                        content: reasoning_content,
                        details: reasoning_details,
                    });
                }
            }
            DecodedState::Part(PartState::ToolCall { call, legacy }) => {
                if let ConversationPart::ToolCall { call_id, .. } = part {
                    if legacy && replay.legacy.is_none() {
                        replay.legacy = Some(LegacyFunctionCall {
                            tool_call_id: call_id.clone(),
                            name: call.name,
                            arguments: call.arguments,
                        });
                    } else {
                        replay.calls.push((call_id.clone(), call));
                    }
                }
            }
            DecodedState::LegacyCall {
                tool_call_id,
                name,
                arguments,
            } => {
                if matches!(part, ConversationPart::ToolCall { call_id, .. } if *call_id == tool_call_id)
                    && replay.legacy.is_none()
                {
                    replay.legacy = Some(LegacyFunctionCall {
                        tool_call_id,
                        name,
                        arguments,
                    });
                }
            }
        }
    }
    if !has_text_state {
        replay.text = None;
    }
    Ok(replay)
}
