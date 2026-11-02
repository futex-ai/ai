//! Provider-owned state built from validated response fields.

use super::{super::dialect::ReplayRule, types::ChatCompletionsToolCall};
use ai_interface::{DeepSeekToolCallContext, KimiToolCallContext, ProviderConversationItem};

pub(super) fn provider_context(
    rule: ReplayRule,
    dispatchable: bool,
    content: Option<String>,
    reasoning_content: Option<String>,
    calls: &[ChatCompletionsToolCall],
) -> Vec<ProviderConversationItem> {
    match rule {
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
    }
}
