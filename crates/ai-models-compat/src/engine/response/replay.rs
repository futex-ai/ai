//! Provider-owned state built from validated response fields.

use super::{super::dialect::ReplayRule, types::ChatCompletionsToolCall};
use ai_interface::{
    DeepSeekToolCallContext, KimiToolCallContext, MiniMaxReasoningDetail, ProviderConversationItem,
    QwenToolCallContext,
};

pub(super) fn provider_context(
    rule: ReplayRule,
    dispatchable: bool,
    content: Option<String>,
    reasoning_content: Option<String>,
    calls: &[ChatCompletionsToolCall],
    reasoning_details: Vec<MiniMaxReasoningDetail>,
) -> Vec<ProviderConversationItem> {
    match rule {
        ReplayRule::MiniMax if reasoning_content.is_none() && reasoning_details.is_empty() => {
            Vec::new()
        }
        ReplayRule::MiniMax => vec![ProviderConversationItem::MiniMaxAssistant {
            reasoning_content,
            reasoning_details,
        }],
        ReplayRule::DeepSeek if dispatchable => {
            vec![ProviderConversationItem::DeepSeekAssistantMessage {
                content: content.unwrap_or_default(),
                reasoning_content,
                tool_calls: calls
                    .iter()
                    .map(|call| DeepSeekToolCallContext {
                        id: call.id.clone(),
                        name: call.function.name.clone(),
                        arguments: call.function.arguments.clone(),
                    })
                    .collect(),
            }]
        }
        ReplayRule::DeepSeek => Vec::new(),
        ReplayRule::Kimi => vec![ProviderConversationItem::KimiAssistantMessage {
            content,
            reasoning_content,
            tool_calls: calls
                .iter()
                .map(|call| KimiToolCallContext {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.clone(),
                })
                .collect(),
        }],
        ReplayRule::Qwen => vec![ProviderConversationItem::QwenAssistantMessage {
            content,
            reasoning_content,
            tool_calls: calls
                .iter()
                .map(|call| QwenToolCallContext {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.clone(),
                })
                .collect(),
        }],
    }
}
