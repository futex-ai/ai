//! xAI chat-completions request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest,
    ProviderConversationItem, StructuredOutputSchema, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::request_types::{
    ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
    ChatCompletionsJsonSchema, ChatCompletionsMessage, ChatCompletionsRequest,
    ChatCompletionsResponseFormat, ChatCompletionsTool, ChatCompletionsToolCall,
    ChatCompletionsToolDefinition, ChatCompletionsToolFunction,
};

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ChatCompletionsRequest {
    let mut messages = vec![ChatCompletionsMessage {
        role: "system".to_owned(),
        content: Some(ChatCompletionsContent::Text(request.system_prompt.clone())),
        name: None,
        tool_call_id: None,
        tool_calls: Vec::new(),
        function_call: None,
    }];
    messages.extend(conversation_messages(&request.messages));

    ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: (!request.tools.is_empty()).then(|| "auto".to_owned()),
        response_format: request.response_schema.as_ref().map(response_format),
        reasoning_effort: reasoning_effort(thinking_level).map(str::to_owned),
    }
}

#[derive(Clone, Debug)]
struct LegacyFunctionCall {
    tool_call_id: String,
    name: String,
    arguments: String,
}

fn conversation_messages(messages: &[ConversationMessage]) -> Vec<ChatCompletionsMessage> {
    let mut legacy_calls = Vec::new();
    let mut output = Vec::new();

    for conversation_message in messages {
        output.push(chat_message(conversation_message, &legacy_calls));
        if let Some(function_call) = legacy_function_call(conversation_message) {
            legacy_calls.push(function_call);
        }
    }

    output
}

fn chat_message(
    message: &ConversationMessage,
    legacy_calls: &[LegacyFunctionCall],
) -> ChatCompletionsMessage {
    let legacy_function_call = legacy_function_call(message);
    let legacy_tool_name = legacy_tool_name(message, legacy_calls);
    ChatCompletionsMessage {
        role: message_role(message.role, legacy_tool_name.is_some()).to_owned(),
        content: message_content(message),
        name: message_name(message, legacy_tool_name),
        tool_call_id: message_tool_call_id(message, legacy_tool_name),
        tool_calls: message_tool_calls(message, legacy_function_call.is_some()),
        function_call: legacy_function_call.map(|function_call| ChatCompletionsToolFunction {
            name: function_call.name,
            arguments: function_call.arguments,
        }),
    }
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

fn message_content(message: &ConversationMessage) -> Option<ChatCompletionsContent> {
    if !message.content_parts.is_empty() {
        return Some(ChatCompletionsContent::Parts(
            message.content_parts.iter().map(content_part).collect(),
        ));
    }
    if message.content.is_empty() {
        None
    } else {
        Some(ChatCompletionsContent::Text(message.content.clone()))
    }
}

fn content_part(part: &ConversationContentPart) -> ChatCompletionsContentPart {
    match part {
        ConversationContentPart::Text { text } => {
            ChatCompletionsContentPart::Text { text: text.clone() }
        }
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => ChatCompletionsContentPart::ImageUrl {
            image_url: ChatCompletionsImageUrl {
                url: format!("data:{mime_type};base64,{data_base64}"),
            },
        },
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
        ProviderConversationItem::OpenAiMessage { .. }
        | ProviderConversationItem::OpenAiReasoning { .. }
        | ProviderConversationItem::OpenAiFunctionCall { .. }
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

fn reasoning_effort(thinking_level: ThinkingLevel) -> Option<&'static str> {
    match thinking_level {
        ThinkingLevel::Disabled => None,
        ThinkingLevel::Low => Some("low"),
        ThinkingLevel::Medium => Some("medium"),
        ThinkingLevel::High => Some("high"),
        ThinkingLevel::ExtraHigh => Some("xhigh"),
        ThinkingLevel::Max => Some("high"),
    }
}
