//! OpenAI Responses output item parsing.

use ai_interface::{ModelError, ProviderConversationItem, ToolCall};
use ai_models_core::parse_tool_call_arguments;

use super::PROVIDER;
use super::types::{ResponsesContentPart, ResponsesOutputItem};

pub(super) fn assistant_message(output: &[ResponsesOutputItem]) -> String {
    output
        .iter()
        .flat_map(|item| match item {
            ResponsesOutputItem::Message { content } => content
                .iter()
                .filter_map(|part| match part {
                    ResponsesContentPart::OutputText { text } => Some(text.clone()),
                    ResponsesContentPart::Refusal { refusal } => Some(refusal.clone()),
                    ResponsesContentPart::Other => None,
                })
                .collect::<Vec<_>>(),
            ResponsesOutputItem::Reasoning { .. }
            | ResponsesOutputItem::FunctionCall { .. }
            | ResponsesOutputItem::Other => Vec::new(),
        })
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn provider_context(output: &[ResponsesOutputItem]) -> Vec<ProviderConversationItem> {
    output
        .iter()
        .filter_map(|item| match item {
            ResponsesOutputItem::Reasoning {
                id,
                summary,
                encrypted_content,
            } => Some(ProviderConversationItem::OpenAiReasoning {
                id: id.clone(),
                summary: summary.clone(),
                encrypted_content: encrypted_content.clone(),
            }),
            ResponsesOutputItem::Message { .. }
            | ResponsesOutputItem::FunctionCall { .. }
            | ResponsesOutputItem::Other => None,
        })
        .collect()
}

pub(super) fn has_function_calls(output: &[ResponsesOutputItem]) -> bool {
    output
        .iter()
        .any(|item| matches!(item, ResponsesOutputItem::FunctionCall { .. }))
}

pub(super) fn tool_calls(
    provider_model_id: &str,
    output: &[ResponsesOutputItem],
) -> std::result::Result<Vec<ToolCall>, ModelError> {
    output
        .iter()
        .filter_map(|item| match item {
            ResponsesOutputItem::FunctionCall {
                call_id,
                name,
                arguments,
            } => Some((call_id, name, arguments)),
            ResponsesOutputItem::Message { .. }
            | ResponsesOutputItem::Reasoning { .. }
            | ResponsesOutputItem::Other => None,
        })
        .map(|(call_id, name, arguments)| {
            Ok(ToolCall {
                id: call_id.clone(),
                name: name.clone(),
                input: parse_tool_call_arguments(PROVIDER, provider_model_id, arguments)?,
                operation_id: None,
            })
        })
        .collect()
}
