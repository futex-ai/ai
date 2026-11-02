//! OpenAI Responses output item parsing.

use ai_interface::ProviderConversationItem;

use super::types::ResponsesOutputItem;

pub(super) fn provider_context(
    output: &[ResponsesOutputItem],
    preserve_function_calls: bool,
) -> Vec<ProviderConversationItem> {
    output
        .iter()
        .filter_map(|item| match item {
            ResponsesOutputItem::Message { phase, .. }
                if preserve_function_calls || phase.is_some() =>
            {
                Some(ProviderConversationItem::OpenAiMessage {
                    phase: phase.clone(),
                })
            }
            ResponsesOutputItem::Reasoning {
                id,
                summary,
                encrypted_content,
            } => Some(ProviderConversationItem::OpenAiReasoning {
                id: id.clone(),
                summary: summary.clone(),
                encrypted_content: encrypted_content.clone(),
            }),
            ResponsesOutputItem::FunctionCall {
                id,
                call_id,
                name,
                arguments,
            } if preserve_function_calls => Some(ProviderConversationItem::OpenAiFunctionCall {
                id: id.clone(),
                call_id: call_id.clone(),
                name: name.clone(),
                arguments: arguments.clone(),
            }),
            ResponsesOutputItem::FunctionCall { .. } | ResponsesOutputItem::Message { .. } => None,
            ResponsesOutputItem::Other => None,
        })
        .collect()
}

pub(super) fn has_function_calls(output: &[ResponsesOutputItem]) -> bool {
    output
        .iter()
        .any(|item| matches!(item, ResponsesOutputItem::FunctionCall { .. }))
}
