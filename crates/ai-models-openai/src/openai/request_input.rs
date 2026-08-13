//! Conversation-to-Responses input item mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelError, ModelResult,
    ProviderConversationItem, ToolCall,
};

use super::request_types::{
    ResponsesContentPart, ResponsesFunctionCallInput, ResponsesFunctionCallOutput,
    ResponsesInputItem, ResponsesMessage, ResponsesMessageContent, ResponsesReasoningInput,
};

const PROVIDER: &str = "openai";

pub(super) fn input_items(
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<ResponsesInputItem>> {
    let mut items = Vec::new();
    for message in messages {
        items.extend(message_items(model_id, message)?);
    }
    Ok(items)
}

fn message_items(
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<Vec<ResponsesInputItem>> {
    match message.role {
        ConversationRole::User => Ok(vec![ResponsesInputItem::Message(message_item(
            model_id, message, "user",
        )?)]),
        ConversationRole::Assistant => assistant_items(model_id, message),
        ConversationRole::Tool => Ok(message
            .tool_call_id
            .as_ref()
            .map(|call_id| {
                vec![ResponsesInputItem::FunctionCallOutput(
                    ResponsesFunctionCallOutput {
                        kind: "function_call_output".to_owned(),
                        call_id: call_id.clone(),
                        output: message.content.clone(),
                    },
                )]
            })
            .unwrap_or_default()),
    }
}

fn assistant_items(
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<Vec<ResponsesInputItem>> {
    let mut items = Vec::new();
    let mut assistant_message_emitted = false;
    for item in &message.provider_context {
        match item {
            ProviderConversationItem::OpenAiMessage { phase } => {
                if has_message_content(message) && !assistant_message_emitted {
                    items.push(ResponsesInputItem::Message(message_item_with_phase(
                        model_id,
                        message,
                        "assistant",
                        phase.clone(),
                    )?));
                    assistant_message_emitted = true;
                }
            }
            ProviderConversationItem::OpenAiReasoning { .. }
            | ProviderConversationItem::OpenAiFunctionCall { .. } => {
                if let Some(provider_item) = provider_context_item(item) {
                    items.push(provider_item);
                }
            }
            ProviderConversationItem::DeepSeekAssistantMessage { .. }
            | ProviderConversationItem::KimiAssistantMessage { .. }
            | ProviderConversationItem::MiniMaxAssistant { .. }
            | ProviderConversationItem::QwenAssistantMessage { .. }
            | ProviderConversationItem::XaiLegacyFunctionCall { .. } => {}
        }
    }
    if has_message_content(message) && !assistant_message_emitted {
        items.push(ResponsesInputItem::Message(message_item(
            model_id,
            message,
            "assistant",
        )?));
    }
    if !has_openai_function_call_context(message) {
        items.extend(
            message
                .tool_calls
                .iter()
                .map(|call| ResponsesInputItem::FunctionCall(function_call_item(call))),
        );
    }
    Ok(items)
}

fn provider_context_item(item: &ProviderConversationItem) -> Option<ResponsesInputItem> {
    match item {
        ProviderConversationItem::DeepSeekAssistantMessage { .. }
        | ProviderConversationItem::OpenAiMessage { .. }
        | ProviderConversationItem::KimiAssistantMessage { .. }
        | ProviderConversationItem::MiniMaxAssistant { .. }
        | ProviderConversationItem::QwenAssistantMessage { .. }
        | ProviderConversationItem::XaiLegacyFunctionCall { .. } => None,
        ProviderConversationItem::OpenAiReasoning {
            id,
            summary,
            encrypted_content,
        } => Some(ResponsesInputItem::Reasoning(ResponsesReasoningInput {
            kind: "reasoning".to_owned(),
            id: id.clone(),
            summary: summary.clone(),
            encrypted_content: encrypted_content.clone(),
        })),
        ProviderConversationItem::OpenAiFunctionCall {
            id,
            call_id,
            name,
            arguments,
        } => Some(ResponsesInputItem::FunctionCall(
            ResponsesFunctionCallInput {
                kind: "function_call".to_owned(),
                id: id.clone(),
                call_id: call_id.clone(),
                name: name.clone(),
                arguments: arguments.clone(),
            },
        )),
    }
}

fn has_openai_function_call_context(message: &ConversationMessage) -> bool {
    message
        .provider_context
        .iter()
        .any(|item| matches!(item, ProviderConversationItem::OpenAiFunctionCall { .. }))
}

fn message_item(
    model_id: &str,
    message: &ConversationMessage,
    role: &str,
) -> ModelResult<ResponsesMessage> {
    message_item_with_phase(model_id, message, role, None)
}

fn message_item_with_phase(
    model_id: &str,
    message: &ConversationMessage,
    role: &str,
    phase: Option<String>,
) -> ModelResult<ResponsesMessage> {
    Ok(ResponsesMessage {
        role: role.to_owned(),
        phase,
        content: message_content(model_id, message)?,
    })
}

fn has_message_content(message: &ConversationMessage) -> bool {
    !message.content.trim().is_empty() || !message.content_parts.is_empty()
}

fn message_content(
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<ResponsesMessageContent> {
    if message.content_parts.is_empty() {
        return Ok(ResponsesMessageContent::Text(message.content.clone()));
    }
    Ok(ResponsesMessageContent::Parts(
        message
            .content_parts
            .iter()
            .map(|part| content_part(model_id, part))
            .collect::<ModelResult<Vec<_>>>()?,
    ))
}

fn content_part(
    model_id: &str,
    part: &ConversationContentPart,
) -> ModelResult<ResponsesContentPart> {
    match part {
        ConversationContentPart::Text { text } => {
            Ok(ResponsesContentPart::InputText { text: text.clone() })
        }
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => Ok(ResponsesContentPart::InputImage {
            image_url: format!("data:{mime_type};base64,{data_base64}"),
        }),
        ConversationContentPart::Video { .. } => Err(ModelError::provider(
            PROVIDER,
            model_id,
            "OpenAI accepts text and image content parts only",
        )),
    }
}

fn function_call_item(call: &ToolCall) -> ResponsesFunctionCallInput {
    ResponsesFunctionCallInput {
        kind: "function_call".to_owned(),
        id: None,
        call_id: call.id.clone(),
        name: call.name.clone(),
        arguments: call.input.to_string(),
    }
}
