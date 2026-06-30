//! xAI chat-completions request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest,
    StructuredOutputSchema, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsRequest {
    model: String,
    messages: Vec<ChatCompletionsMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ChatCompletionsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ChatCompletionsResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsMessage {
    role: String,
    content: Option<ChatCompletionsContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ChatCompletionsToolCall>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ChatCompletionsContent {
    Text(String),
    Parts(Vec<ChatCompletionsContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ChatCompletionsContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ChatCompletionsImageUrl },
}

#[derive(Debug, Serialize)]
struct ChatCompletionsImageUrl {
    url: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: ChatCompletionsToolFunction,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsTool {
    #[serde(rename = "type")]
    kind: String,
    function: ChatCompletionsToolDefinition,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsToolDefinition {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsToolFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsResponseFormat {
    #[serde(rename = "type")]
    kind: String,
    json_schema: ChatCompletionsJsonSchema,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsJsonSchema {
    name: String,
    schema: Value,
    strict: bool,
}

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
    }];
    messages.extend(request.messages.iter().map(message));

    ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: (!request.tools.is_empty()).then(|| "auto".to_owned()),
        response_format: request.response_schema.as_ref().map(response_format),
        reasoning_effort: reasoning_effort(thinking_level).map(str::to_owned),
    }
}

fn message(message: &ConversationMessage) -> ChatCompletionsMessage {
    ChatCompletionsMessage {
        role: match message.role {
            ConversationRole::User => "user",
            ConversationRole::Assistant => "assistant",
            ConversationRole::Tool => "tool",
        }
        .to_owned(),
        content: message_content(message),
        name: message_name(message),
        tool_call_id: message.tool_call_id.clone(),
        tool_calls: message.tool_calls.iter().map(tool_call).collect(),
    }
}

fn message_name(message: &ConversationMessage) -> Option<String> {
    match message.role {
        ConversationRole::Tool => None,
        ConversationRole::User | ConversationRole::Assistant => message.name.clone(),
    }
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

fn tool_call(call: &ai_interface::ToolCall) -> ChatCompletionsToolCall {
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
