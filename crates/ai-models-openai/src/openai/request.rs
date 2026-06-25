//! OpenAI Responses request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest,
    StructuredOutputSchema, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct ResponsesRequest {
    model: String,
    instructions: String,
    input: Vec<ResponsesInputItem>,
    store: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ResponsesTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<ResponsesText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ResponsesReasoning>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ResponsesInputItem {
    Message(ResponsesMessage),
    FunctionCall(ResponsesFunctionCallInput),
    FunctionCallOutput(ResponsesFunctionCallOutput),
}

#[derive(Debug, Serialize)]
struct ResponsesMessage {
    role: String,
    content: ResponsesMessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ResponsesMessageContent {
    Text(String),
    Parts(Vec<ResponsesContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ResponsesContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

#[derive(Debug, Serialize)]
struct ResponsesFunctionCallInput {
    #[serde(rename = "type")]
    kind: String,
    call_id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct ResponsesFunctionCallOutput {
    #[serde(rename = "type")]
    kind: String,
    call_id: String,
    output: String,
}

#[derive(Debug, Serialize)]
struct ResponsesTool {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    description: String,
    parameters: Value,
    strict: bool,
}

#[derive(Debug, Serialize)]
struct ResponsesText {
    format: ResponsesTextFormat,
}

#[derive(Debug, Serialize)]
struct ResponsesTextFormat {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    strict: bool,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct ResponsesReasoning {
    effort: String,
}

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ResponsesRequest {
    let tools = request.tools.iter().map(tool).collect::<Vec<_>>();
    ResponsesRequest {
        model: model_id.to_owned(),
        instructions: request.system_prompt.clone(),
        input: input_items(&request.messages),
        store: false,
        tool_choice: (!tools.is_empty()).then(|| "auto".to_owned()),
        tools,
        text: request.response_schema.as_ref().map(text_format),
        reasoning: reasoning(thinking_level),
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
    if !message.content.trim().is_empty() || !message.content_parts.is_empty() {
        items.push(ResponsesInputItem::Message(message_item(
            message,
            "assistant",
        )));
    }
    items.extend(
        message
            .tool_calls
            .iter()
            .map(|call| ResponsesInputItem::FunctionCall(function_call_item(call))),
    );
    items
}

fn message_item(message: &ConversationMessage, role: &str) -> ResponsesMessage {
    ResponsesMessage {
        role: role.to_owned(),
        content: message_content(message),
    }
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
            strict: true,
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
        ThinkingLevel::ExtraHigh | ThinkingLevel::Max => "xhigh",
    };
    Some(ResponsesReasoning {
        effort: effort.to_owned(),
    })
}
