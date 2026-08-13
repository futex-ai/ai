//! Shared request-to-Kimi Chat Completions mapping.

use super::client::KimiReasoningEffort;
use super::request_types::{
    ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
    ChatCompletionsJsonSchema, ChatCompletionsMessage, ChatCompletionsNamedFunction,
    ChatCompletionsNamedToolChoice, ChatCompletionsRequest, ChatCompletionsResponseFormat,
    ChatCompletionsTool, ChatCompletionsToolCall, ChatCompletionsToolChoice,
    ChatCompletionsToolDefinition, ChatCompletionsToolFunction,
};
use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, KimiToolCallContext,
    ModelError, ModelRequest, ModelResult, ModelToolChoice, ProviderConversationItem,
    StructuredOutputSchema, ToolCall, ToolDefinition,
};

const PROVIDER: &str = "kimi";

pub(super) fn build_request(
    model_id: &str,
    reasoning_effort: KimiReasoningEffort,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    let mut messages = Vec::new();
    if let Some(system_prompt) = request.nonblank_system_prompt() {
        messages.push(ChatCompletionsMessage {
            role: "system".to_owned(),
            content: Some(ChatCompletionsContent::Text(system_prompt.to_owned())),
            name: None,
            tool_call_id: None,
            reasoning_content: None,
            tool_calls: Vec::new(),
        });
    }
    for conversation_message in &request.messages {
        messages.push(message(model_id, conversation_message)?);
    }
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: tool_choice(request, !request.tools.is_empty()),
        response_format: request.response_schema.as_ref().map(response_format),
        reasoning_effort: reasoning_effort.as_str().to_owned(),
        max_completion_tokens: request.controls.generation.max_output_tokens,
        stop: request.controls.generation.stop_sequences.clone(),
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

fn message(model_id: &str, message: &ConversationMessage) -> ModelResult<ChatCompletionsMessage> {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = kimi_replay(message)
    {
        return Ok(replay_message(replay));
    }
    Ok(ChatCompletionsMessage {
        role: role(message.role).to_owned(),
        content: message_content(model_id, message)?,
        name: message_name(message),
        tool_call_id: message.tool_call_id.clone(),
        reasoning_content: None,
        tool_calls: if message.role == ConversationRole::Assistant {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
    })
}

fn kimi_replay(message: &ConversationMessage) -> Option<&ProviderConversationItem> {
    message
        .provider_context
        .iter()
        .find(|item| matches!(item, ProviderConversationItem::KimiAssistantMessage { .. }))
}

fn replay_message(item: &ProviderConversationItem) -> ChatCompletionsMessage {
    let ProviderConversationItem::KimiAssistantMessage {
        content,
        reasoning_content,
        tool_calls,
    } = item
    else {
        return ChatCompletionsMessage {
            role: "assistant".to_owned(),
            content: None,
            name: None,
            tool_call_id: None,
            reasoning_content: None,
            tool_calls: Vec::new(),
        };
    };
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: content.clone().map(ChatCompletionsContent::Text),
        name: None,
        tool_call_id: None,
        reasoning_content: reasoning_content.clone(),
        tool_calls: tool_calls.iter().map(raw_tool_call).collect(),
    }
}

fn role(role: ConversationRole) -> &'static str {
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}

fn message_name(message: &ConversationMessage) -> Option<String> {
    match message.role {
        ConversationRole::User | ConversationRole::Assistant => message.name.clone(),
        ConversationRole::Tool => None,
    }
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
    if message.role == ConversationRole::Assistant && message.content.is_empty() {
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
            "Kimi accepts text and image content parts only",
        )),
    }
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

fn raw_tool_call(call: &KimiToolCallContext) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
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

fn response_format(schema: &StructuredOutputSchema) -> ChatCompletionsResponseFormat {
    ChatCompletionsResponseFormat {
        kind: "json_schema".to_owned(),
        json_schema: ChatCompletionsJsonSchema {
            name: schema.name.clone(),
            schema: schema.schema.clone(),
            strict: false,
        },
    }
}
