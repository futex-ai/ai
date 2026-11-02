//! Provider-owned replay data attached only to the part that produced it.

use ai_interface::{ConversationPart, ModelResult, ToolCall};

use crate::minimax::state::ReasoningDetail;

use super::{
    super::{
        dialect::{CompletionContext, ReplayRule},
        replay_state::{self, PartState, RawToolCall},
    },
    types::ChatCompletionsToolCall,
};

pub(super) struct ResponseContent {
    pub(super) text: String,
    pub(super) raw_content: Option<String>,
    pub(super) reasoning: Option<String>,
    pub(super) details: Vec<ReasoningDetail>,
    pub(super) calls: Vec<ToolCall>,
    pub(super) raw_calls: Vec<ChatCompletionsToolCall>,
    pub(super) legacy: bool,
    pub(super) dispatchable: bool,
}

pub(super) fn parts(
    context: CompletionContext<'_>,
    content: ResponseContent,
) -> ModelResult<Vec<ConversationPart>> {
    let mut parts = Vec::new();
    let rule = context.dialect.replay;
    let retains_assistant = matches!(rule, ReplayRule::Kimi | ReplayRule::Qwen)
        || rule == ReplayRule::DeepSeek && content.dispatchable;
    let retains_reasoning = retains_assistant
        || rule == ReplayRule::MiniMax
            && (content.reasoning.is_some() || !content.details.is_empty());
    if content
        .reasoning
        .as_ref()
        .is_some_and(|text| !text.is_empty())
        || retains_reasoning
    {
        let mut part = ConversationPart::reasoning(content.reasoning.clone().unwrap_or_default());
        if retains_reasoning {
            part = part.with_provider_state(replay_state::encode(
                context.dialect,
                context.provider_model_id,
                PartState::Reasoning {
                    reasoning_content: content.reasoning,
                    reasoning_details: content.details,
                },
            )?);
        }
        parts.push(part);
    }
    if !content.text.is_empty() || retains_assistant {
        let mut part = ConversationPart::text(content.text);
        if retains_assistant {
            let raw = if rule == ReplayRule::DeepSeek {
                Some(content.raw_content.unwrap_or_default())
            } else {
                content.raw_content
            };
            part = part.with_provider_state(replay_state::encode(
                context.dialect,
                context.provider_model_id,
                PartState::Text { content: raw },
            )?);
        }
        parts.push(part);
    }
    for (call, raw) in content.calls.into_iter().zip(content.raw_calls) {
        parts.push(ConversationPart::from_tool_call(call).with_provider_state(
            replay_state::encode(
                context.dialect,
                context.provider_model_id,
                PartState::ToolCall {
                    call: RawToolCall {
                        id: raw.id,
                        name: raw.function.name,
                        arguments: raw.function.arguments,
                    },
                    legacy: content.legacy,
                },
            )?,
        ));
    }
    Ok(parts)
}
