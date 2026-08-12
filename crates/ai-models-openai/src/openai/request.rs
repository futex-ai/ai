//! OpenAI Responses request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest, ModelToolChoice,
    ProviderConversationItem, StructuredOutputSchema, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::request_types::{
    ResponsesContentPart, ResponsesFunctionCallInput, ResponsesFunctionCallOutput,
    ResponsesInputItem, ResponsesMessage, ResponsesMessageContent, ResponsesNamedToolChoice,
    ResponsesReasoning, ResponsesReasoningInput, ResponsesRequest, ResponsesText,
    ResponsesTextFormat, ResponsesTool, ResponsesToolChoice,
};

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ResponsesRequest {
    let tools = request.tools.iter().map(tool).collect::<Vec<_>>();
    ResponsesRequest {
        model: model_id.to_owned(),
        instructions: request.nonblank_system_prompt().map(str::to_owned),
        input: input_items(&request.messages),
        store: false,
        include: include_items(thinking_level),
        tool_choice: tool_choice(request, !tools.is_empty()),
        tools,
        text: request.response_schema.as_ref().map(text_format),
        reasoning: reasoning(thinking_level),
        temperature: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.temperature)
            .flatten(),
        top_p: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.top_p)
            .flatten(),
        max_output_tokens: request.controls.generation.max_output_tokens,
    }
}

fn tool_choice(request: &ModelRequest, has_tools: bool) -> Option<ResponsesToolChoice> {
    match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::None) => Some(ResponsesToolChoice::Mode("none".to_owned())),
        Some(ModelToolChoice::Auto) => Some(ResponsesToolChoice::Mode("auto".to_owned())),
        Some(ModelToolChoice::Required | ModelToolChoice::RequiredOrAuto) => {
            Some(ResponsesToolChoice::Mode("required".to_owned()))
        }
        Some(ModelToolChoice::Function(name)) => {
            Some(ResponsesToolChoice::Function(ResponsesNamedToolChoice {
                kind: "function".to_owned(),
                name: name.clone(),
            }))
        }
        None if has_tools => Some(ResponsesToolChoice::Mode("auto".to_owned())),
        None => None,
    }
}

fn input_items(messages: &[ConversationMessage]) -> Vec<ResponsesInputItem> {
    messages.iter().flat_map(message_items).collect()
}

fn message_items(message: &ConversationMessage) -> Vec<ResponsesInputItem> {
    match message.role {
        ConversationRole::User => vec![ResponsesInputItem::Message(message_item(message, "user"))],
        ConversationRole::Assistant => assistant_items(message),
        ConversationRole::Tool => message
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
            .unwrap_or_default(),
    }
}

fn assistant_items(message: &ConversationMessage) -> Vec<ResponsesInputItem> {
    let mut items = Vec::new();
    let mut assistant_message_emitted = false;
    for item in &message.provider_context {
        match item {
            ProviderConversationItem::OpenAiMessage { phase } => {
                if has_message_content(message) && !assistant_message_emitted {
                    items.push(ResponsesInputItem::Message(message_item_with_phase(
                        message,
                        "assistant",
                        phase.clone(),
                    )));
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
            message,
            "assistant",
        )));
    }
    if !has_openai_function_call_context(message) {
        items.extend(
            message
                .tool_calls
                .iter()
                .map(|call| ResponsesInputItem::FunctionCall(function_call_item(call))),
        );
    }
    items
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

fn message_item(message: &ConversationMessage, role: &str) -> ResponsesMessage {
    message_item_with_phase(message, role, None)
}

fn message_item_with_phase(
    message: &ConversationMessage,
    role: &str,
    phase: Option<String>,
) -> ResponsesMessage {
    ResponsesMessage {
        role: role.to_owned(),
        phase,
        content: message_content(message),
    }
}

fn has_message_content(message: &ConversationMessage) -> bool {
    !message.content.trim().is_empty() || !message.content_parts.is_empty()
}

fn message_content(message: &ConversationMessage) -> ResponsesMessageContent {
    if message.content_parts.is_empty() {
        return ResponsesMessageContent::Text(message.content.clone());
    }
    ResponsesMessageContent::Parts(message.content_parts.iter().map(content_part).collect())
}

fn content_part(part: &ConversationContentPart) -> ResponsesContentPart {
    match part {
        ConversationContentPart::Text { text } => {
            ResponsesContentPart::InputText { text: text.clone() }
        }
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => ResponsesContentPart::InputImage {
            image_url: format!("data:{mime_type};base64,{data_base64}"),
        },
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

fn tool(tool: &ToolDefinition) -> ResponsesTool {
    ResponsesTool {
        kind: "function".to_owned(),
        name: tool.name.clone(),
        description: tool.description.clone(),
        parameters: tool.input_schema.clone(),
        strict: false,
    }
}

fn text_format(response_schema: &StructuredOutputSchema) -> ResponsesText {
    ResponsesText {
        format: ResponsesTextFormat {
            kind: "json_schema".to_owned(),
            name: response_schema.name.clone(),
            strict: false,
            schema: response_schema.schema.clone(),
        },
    }
}

fn reasoning(thinking_level: ThinkingLevel) -> Option<ResponsesReasoning> {
    let effort = match thinking_level {
        ThinkingLevel::Disabled => return None,
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
        ThinkingLevel::ExtraHigh => "xhigh",
        ThinkingLevel::Max => "max",
    };
    Some(ResponsesReasoning {
        effort: effort.to_owned(),
    })
}

fn include_items(thinking_level: ThinkingLevel) -> Vec<String> {
    thinking_level
        .is_enabled()
        .then(|| "reasoning.encrypted_content".to_owned())
        .into_iter()
        .collect()
}
