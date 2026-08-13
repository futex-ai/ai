//! xAI chat-completions request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelCompletionMode,
    ModelError, ModelRequest, ModelResult, ModelToolChoice, ProviderConversationItem,
    StructuredOutputSchema, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::request_types::{
    ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
    ChatCompletionsJsonSchema, ChatCompletionsMessage, ChatCompletionsNamedFunction,
    ChatCompletionsNamedToolChoice, ChatCompletionsRequest, ChatCompletionsResponseFormat,
    ChatCompletionsTool, ChatCompletionsToolCall, ChatCompletionsToolChoice,
    ChatCompletionsToolDefinition, ChatCompletionsToolFunction,
};

const PROVIDER: &str = "xai";

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    let mut messages = Vec::new();
    if let Some(system_prompt) = request.nonblank_system_prompt() {
        messages.push(ChatCompletionsMessage {
            role: "system".to_owned(),
            content: Some(ChatCompletionsContent::Text(system_prompt.to_owned())),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            function_call: None,
        });
    }
    messages.extend(conversation_messages(model_id, &request.messages)?);

    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: tool_choice(request, !request.tools.is_empty()),
        response_format: request.response_schema.as_ref().map(response_format),
        reasoning_effort: reasoning_effort(model_id, thinking_level).map(str::to_owned),
        temperature: request.controls.generation.temperature,
        top_p: request.controls.generation.top_p,
        max_tokens: request.controls.generation.max_output_tokens,
        stop: request.controls.generation.stop_sequences.clone(),
        deferred: request.controls.execution.completion_mode != ModelCompletionMode::Synchronous,
    })
}

fn tool_choice(request: &ModelRequest, has_tools: bool) -> Option<ChatCompletionsToolChoice> {
    match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::None) => Some(ChatCompletionsToolChoice::Mode("none".to_owned())),
        Some(ModelToolChoice::Auto) => Some(ChatCompletionsToolChoice::Mode("auto".to_owned())),
        Some(ModelToolChoice::Required | ModelToolChoice::RequiredOrAuto) => {
            Some(ChatCompletionsToolChoice::Mode("required".to_owned()))
        }
        Some(ModelToolChoice::Function(name)) => Some(ChatCompletionsToolChoice::Function(
            ChatCompletionsNamedToolChoice {
                kind: "function".to_owned(),
                function: ChatCompletionsNamedFunction { name: name.clone() },
            },
        )),
        None if has_tools => Some(ChatCompletionsToolChoice::Mode("auto".to_owned())),
        None => None,
    }
}

#[derive(Clone, Debug)]
struct LegacyFunctionCall {
    tool_call_id: String,
    name: String,
    arguments: String,
}

fn conversation_messages(
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<ChatCompletionsMessage>> {
    let mut legacy_calls = Vec::new();
    let mut output = Vec::new();

    for conversation_message in messages {
        output.push(chat_message(model_id, conversation_message, &legacy_calls)?);
        if let Some(function_call) = legacy_function_call(conversation_message) {
            legacy_calls.push(function_call);
        }
    }

    Ok(output)
}

fn chat_message(
    model_id: &str,
    message: &ConversationMessage,
    legacy_calls: &[LegacyFunctionCall],
) -> ModelResult<ChatCompletionsMessage> {
    let legacy_function_call = legacy_function_call(message);
    let legacy_tool_name = legacy_tool_name(message, legacy_calls);
    Ok(ChatCompletionsMessage {
        role: message_role(message.role, legacy_tool_name.is_some()).to_owned(),
        content: message_content(model_id, message)?,
        name: message_name(message, legacy_tool_name),
        tool_call_id: message_tool_call_id(message, legacy_tool_name),
        tool_calls: message_tool_calls(message, legacy_function_call.is_some()),
        function_call: legacy_function_call.map(|function_call| ChatCompletionsToolFunction {
            name: function_call.name,
            arguments: function_call.arguments,
        }),
    })
}

fn message_role(role: ConversationRole, is_legacy_tool_result: bool) -> &'static str {
    if is_legacy_tool_result {
        return "function";
    }
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}

fn message_name(message: &ConversationMessage, legacy_tool_name: Option<&str>) -> Option<String> {
    if let Some(name) = legacy_tool_name {
        return Some(name.to_owned());
    }
    match message.role {
        ConversationRole::Tool => None,
        ConversationRole::User | ConversationRole::Assistant => message.name.clone(),
    }
}

fn message_tool_call_id(
    message: &ConversationMessage,
    legacy_tool_name: Option<&str>,
) -> Option<String> {
    if legacy_tool_name.is_some() {
        return None;
    }
    message.tool_call_id.clone()
}

fn message_content(
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<Option<ChatCompletionsContent>> {
    if !message.content_parts.is_empty() {
        return Ok(Some(ChatCompletionsContent::Parts(
            message
                .content_parts
                .iter()
                .map(|part| content_part(model_id, part))
                .collect::<ModelResult<Vec<_>>>()?,
        )));
    }
    if message.content.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ChatCompletionsContent::Text(message.content.clone())))
    }
}

fn content_part(
    model_id: &str,
    part: &ConversationContentPart,
) -> ModelResult<ChatCompletionsContentPart> {
    match part {
        ConversationContentPart::Text { text } => {
            Ok(ChatCompletionsContentPart::Text { text: text.clone() })
        }
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => Ok(ChatCompletionsContentPart::ImageUrl {
            image_url: ChatCompletionsImageUrl {
                url: format!("data:{mime_type};base64,{data_base64}"),
            },
        }),
        ConversationContentPart::Video { .. } => Err(ModelError::provider(
            PROVIDER,
            model_id,
            "xAI accepts text and image content parts only",
        )),
    }
}

fn message_tool_calls(
    message: &ConversationMessage,
    has_legacy_function_call: bool,
) -> Vec<ChatCompletionsToolCall> {
    if has_legacy_function_call {
        return Vec::new();
    }
    message.tool_calls.iter().map(tool_call).collect()
}

fn legacy_function_call(message: &ConversationMessage) -> Option<LegacyFunctionCall> {
    if message.role != ConversationRole::Assistant {
        return None;
    }
    message.provider_context.iter().find_map(|item| match item {
        ProviderConversationItem::XaiLegacyFunctionCall {
            tool_call_id,
            name,
            arguments,
        } if message
            .tool_calls
            .iter()
            .any(|call| call.id == *tool_call_id) =>
        {
            Some(LegacyFunctionCall {
                tool_call_id: tool_call_id.clone(),
                name: name.clone(),
                arguments: arguments.clone(),
            })
        }
        ProviderConversationItem::DeepSeekAssistantMessage { .. }
        | ProviderConversationItem::OpenAiMessage { .. }
        | ProviderConversationItem::OpenAiReasoning { .. }
        | ProviderConversationItem::OpenAiFunctionCall { .. }
        | ProviderConversationItem::KimiAssistantMessage { .. }
        | ProviderConversationItem::MiniMaxAssistant { .. }
        | ProviderConversationItem::QwenAssistantMessage { .. }
        | ProviderConversationItem::XaiLegacyFunctionCall { .. } => None,
    })
}

fn legacy_tool_name<'a>(
    message: &ConversationMessage,
    legacy_calls: &'a [LegacyFunctionCall],
) -> Option<&'a str> {
    if message.role != ConversationRole::Tool {
        return None;
    }
    let tool_call_id = message.tool_call_id.as_deref()?;
    legacy_calls
        .iter()
        .rev()
        .find(|call| call.tool_call_id == tool_call_id)
        .map(|call| call.name.as_str())
}

fn tool_call(call: &ToolCall) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.input.to_string(),
        },
    }
}

fn tool(tool: &ToolDefinition) -> ChatCompletionsTool {
    ChatCompletionsTool {
        kind: "function".to_owned(),
        function: ChatCompletionsToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        },
    }
}

fn response_format(response_schema: &StructuredOutputSchema) -> ChatCompletionsResponseFormat {
    ChatCompletionsResponseFormat {
        kind: "json_schema".to_owned(),
        json_schema: ChatCompletionsJsonSchema {
            name: response_schema.name.clone(),
            schema: response_schema.schema.clone(),
            strict: false,
        },
    }
}

fn reasoning_effort(model_id: &str, thinking_level: ThinkingLevel) -> Option<&'static str> {
    if model_id.starts_with("grok-4.20") {
        return None;
    }
    match thinking_level {
        ThinkingLevel::Disabled => None,
        ThinkingLevel::Low => Some("low"),
        ThinkingLevel::Medium => Some("medium"),
        ThinkingLevel::High => Some("high"),
        ThinkingLevel::ExtraHigh => Some("xhigh"),
        ThinkingLevel::Max => Some("high"),
    }
}
