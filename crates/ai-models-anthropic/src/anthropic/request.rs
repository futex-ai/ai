//! Anthropic messages request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde::Serialize;
use serde_json::Value;

const ANTHROPIC_MAX_TOKENS: u32 = 4096;

#[derive(Debug, Serialize)]
pub(super) struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<AnthropicTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<AnthropicThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_config: Option<AnthropicOutputConfig>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum AnthropicBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: AnthropicImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct AnthropicImageSource {
    #[serde(rename = "type")]
    kind: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Serialize)]
struct AnthropicThinking {
    #[serde(rename = "type")]
    kind: String,
    display: String,
}

#[derive(Debug, Serialize)]
struct AnthropicOutputConfig {
    effort: String,
}

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> AnthropicRequest {
    AnthropicRequest {
        model: model_id.to_owned(),
        max_tokens: ANTHROPIC_MAX_TOKENS,
        system: system_prompt(request),
        messages: anthropic_messages(&request.messages),
        tools: request.tools.iter().map(tool).collect(),
        thinking: thinking(thinking_level),
        output_config: output_config(thinking_level),
    }
}

fn system_prompt(request: &ModelRequest) -> String {
    let Some(response_schema) = request.response_schema.as_ref() else {
        return request.system_prompt.clone();
    };
    format!(
        "{}\n\nWhen you are ready to provide the final answer, return raw JSON only with no markdown fences or extra prose. The JSON must match schema `{}` exactly.\nSchema: {}",
        request.system_prompt, response_schema.name, response_schema.schema
    )
}

fn anthropic_messages(messages: &[ConversationMessage]) -> Vec<AnthropicMessage> {
    let mut output = Vec::new();

    for message in messages {
        match message.role {
            ConversationRole::User => {
                append_blocks(&mut output, "user", user_blocks(message));
            }
            ConversationRole::Assistant => {
                let mut blocks = Vec::new();
                if !message.content.is_empty() {
                    blocks.push(AnthropicBlock::Text {
                        text: message.content.clone(),
                    });
                }
                blocks.extend(
                    message
                        .tool_calls
                        .iter()
                        .map(|call| AnthropicBlock::ToolUse {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            input: call.input.clone(),
                        }),
                );
                append_blocks(&mut output, "assistant", blocks);
            }
            ConversationRole::Tool => {
                append_blocks(
                    &mut output,
                    "user",
                    vec![AnthropicBlock::ToolResult {
                        tool_use_id: message.tool_call_id.clone(),
                        content: message.content.clone(),
                    }],
                );
            }
        }
    }

    output
}

fn user_blocks(message: &ConversationMessage) -> Vec<AnthropicBlock> {
    if message.content_parts.is_empty() {
        return vec![AnthropicBlock::Text {
            text: message.content.clone(),
        }];
    }
    message.content_parts.iter().map(content_part).collect()
}

fn content_part(part: &ConversationContentPart) -> AnthropicBlock {
    match part {
        ConversationContentPart::Text { text } => AnthropicBlock::Text { text: text.clone() },
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => AnthropicBlock::Image {
            source: AnthropicImageSource {
                kind: "base64".to_owned(),
                media_type: mime_type.clone(),
                data: data_base64.clone(),
            },
        },
    }
}

fn append_blocks(messages: &mut Vec<AnthropicMessage>, role: &str, blocks: Vec<AnthropicBlock>) {
    if blocks.is_empty() {
        return;
    }
    if let Some(existing) = messages.last_mut()
        && existing.role == role
    {
        existing.content.extend(blocks);
        return;
    }
    messages.push(AnthropicMessage {
        role: role.to_owned(),
        content: blocks,
    });
}

fn tool(tool: &ToolDefinition) -> AnthropicTool {
    AnthropicTool {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: tool.input_schema.clone(),
    }
}

fn thinking(thinking_level: ThinkingLevel) -> Option<AnthropicThinking> {
    thinking_level.is_enabled().then(|| AnthropicThinking {
        kind: "adaptive".to_owned(),
        display: "omitted".to_owned(),
    })
}

fn output_config(thinking_level: ThinkingLevel) -> Option<AnthropicOutputConfig> {
    effort(thinking_level).map(|effort| AnthropicOutputConfig {
        effort: effort.to_owned(),
    })
}

fn effort(thinking_level: ThinkingLevel) -> Option<&'static str> {
    match thinking_level {
        ThinkingLevel::Disabled => None,
        ThinkingLevel::Low => Some("low"),
        ThinkingLevel::Medium => Some("medium"),
        ThinkingLevel::High => Some("high"),
        ThinkingLevel::ExtraHigh => Some("xhigh"),
        ThinkingLevel::Max => Some("max"),
    }
}
