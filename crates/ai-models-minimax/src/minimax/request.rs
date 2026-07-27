//! MiniMax chat-completions request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, MiniMaxReasoningDetail,
    ModelRequest, ProviderConversationItem, StructuredOutputSchema, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::request_types::{
    ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
    ChatCompletionsMessage, ChatCompletionsRequest, ChatCompletionsTool, ChatCompletionsToolCall,
    ChatCompletionsToolDefinition, ChatCompletionsToolFunction, ThinkingControl,
};

const STRUCTURED_OUTPUT_INSTRUCTION: &str = "Return only raw JSON that validates against the \
following JSON Schema. Do not use Markdown fences or include additional prose.";

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ChatCompletionsRequest {
    let mut messages = vec![ChatCompletionsMessage {
        role: "system",
        content: Some(ChatCompletionsContent::Text(system_prompt(request))),
        name: None,
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
        reasoning_details: Vec::new(),
    }];
    messages.extend(request.messages.iter().map(chat_message));

    ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        stream: false,
        reasoning_split: true,
        thinking: ThinkingControl {
            kind: if thinking_level.is_enabled() {
                "adaptive"
            } else {
                "disabled"
            },
        },
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: (!request.tools.is_empty()).then_some("auto"),
    }
}

fn chat_message(message: &ConversationMessage) -> ChatCompletionsMessage {
    let (reasoning_content, reasoning_details) = minimax_context(message);
    ChatCompletionsMessage {
        role: message_role(message.role),
        content: message_content(message),
        name: match message.role {
            ConversationRole::Tool => None,
            ConversationRole::User | ConversationRole::Assistant => message.name.clone(),
        },
        tool_call_id: if message.role == ConversationRole::Tool {
            message.tool_call_id.clone()
        } else {
            None
        },
        tool_calls: message.tool_calls.iter().map(tool_call).collect(),
        reasoning_content,
        reasoning_details,
    }
}

fn system_prompt(request: &ModelRequest) -> String {
    match request.response_schema.as_ref() {
        Some(schema) => structured_system_prompt(&request.system_prompt, schema),
        None => request.system_prompt.clone(),
    }
}

fn structured_system_prompt(
    system_prompt: &str,
    response_schema: &StructuredOutputSchema,
) -> String {
    format!(
        "{system_prompt}\n\n{STRUCTURED_OUTPUT_INSTRUCTION}\nSchema name: {}\nJSON Schema:\n{}",
        response_schema.name, response_schema.schema
    )
}

fn message_content(message: &ConversationMessage) -> Option<ChatCompletionsContent> {
    if !message.content_parts.is_empty() {
        return Some(ChatCompletionsContent::Parts(
            message.content_parts.iter().map(content_part).collect(),
        ));
    }
    if message.role == ConversationRole::Tool {
        return Some(ChatCompletionsContent::Text(message.content.clone()));
    }
    (!message.content.is_empty()).then(|| ChatCompletionsContent::Text(message.content.clone()))
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

fn message_role(role: ConversationRole) -> &'static str {
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}

fn minimax_context(message: &ConversationMessage) -> (Option<String>, Vec<MiniMaxReasoningDetail>) {
    if message.role != ConversationRole::Assistant {
        return (None, Vec::new());
    }
    message
        .provider_context
        .iter()
        .find_map(|item| match item {
            ProviderConversationItem::MiniMaxAssistant {
                reasoning_content,
                reasoning_details,
            } => Some((reasoning_content.clone(), reasoning_details.clone())),
            ProviderConversationItem::OpenAiMessage { .. }
            | ProviderConversationItem::OpenAiReasoning { .. }
            | ProviderConversationItem::OpenAiFunctionCall { .. }
            | ProviderConversationItem::KimiAssistantMessage { .. }
            | ProviderConversationItem::XaiLegacyFunctionCall { .. } => None,
        })
        .unwrap_or_default()
}

fn tool_call(call: &ToolCall) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function",
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.input.to_string(),
        },
    }
}

fn tool(tool: &ToolDefinition) -> ChatCompletionsTool {
    ChatCompletionsTool {
        kind: "function",
        function: ChatCompletionsToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        },
    }
}
