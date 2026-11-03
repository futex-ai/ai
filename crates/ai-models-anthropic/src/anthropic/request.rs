//! Anthropic messages request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelControl, ModelError,
    ModelRequest, ModelResult, ModelToolChoice, ProviderConversationItem, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use crate::catalog::known_models;

use super::request_types::{
    AnthropicBlock, AnthropicImageSource, AnthropicMessage, AnthropicOutputConfig,
    AnthropicRequest, AnthropicThinking, AnthropicTool, AnthropicToolChoice,
};

const UNKNOWN_MODEL_MAX_OUTPUT_TOKENS: u32 = 64_000;
const PROVIDER: &str = "anthropic";

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<AnthropicRequest> {
    let max_output_tokens = known_models()
        .into_iter()
        .find(|model| model.provider_model_id == model_id)
        .map_or(UNKNOWN_MODEL_MAX_OUTPUT_TOKENS, |model| {
            model.max_output_tokens
        });
    Ok(AnthropicRequest {
        stream: true,
        model: model_id.to_owned(),
        max_tokens: request
            .controls
            .generation
            .max_output_tokens
            .unwrap_or(max_output_tokens)
            .min(max_output_tokens),
        system: system_prompt(request),
        messages: anthropic_messages(model_id, &request.messages)?,
        tools: request.tools.iter().map(tool).collect(),
        thinking: thinking(thinking_level),
        output_config: output_config(thinking_level),
        temperature: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.temperature)
            .flatten(),
        top_p: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.top_p)
            .flatten(),
        stop_sequences: request.controls.generation.stop_sequences.clone(),
        tool_choice: tool_choice(
            model_id,
            thinking_level,
            request.controls.generation.tool_choice.as_ref(),
        )?,
    })
}

fn system_prompt(request: &ModelRequest) -> Option<String> {
    let Some(response_schema) = request.response_schema.as_ref() else {
        return request.nonblank_system_prompt().map(str::to_owned);
    };
    Some(format!(
        "{}\n\nWhen you are ready to provide the final answer, return raw JSON only with no markdown fences or extra prose. The JSON must match schema `{}` exactly.\nSchema: {}",
        request.system_prompt, response_schema.name, response_schema.schema
    ))
}

fn tool_choice(
    model_id: &str,
    thinking_level: ThinkingLevel,
    choice: Option<&ModelToolChoice>,
) -> ModelResult<Option<AnthropicToolChoice>> {
    if thinking_level.is_enabled() {
        match choice {
            Some(ModelToolChoice::Required | ModelToolChoice::Function(_)) => {
                return Err(ModelError::unsupported_control(
                    PROVIDER,
                    model_id,
                    ModelControl::ToolChoice,
                ));
            }
            Some(ModelToolChoice::RequiredOrAuto) => {
                return Ok(Some(AnthropicToolChoice {
                    kind: "auto".to_owned(),
                    name: None,
                }));
            }
            _ => {}
        }
    }
    Ok(match choice {
        Some(ModelToolChoice::None) => Some(AnthropicToolChoice {
            kind: "none".to_owned(),
            name: None,
        }),
        Some(ModelToolChoice::Auto) => Some(AnthropicToolChoice {
            kind: "auto".to_owned(),
            name: None,
        }),
        Some(ModelToolChoice::Required | ModelToolChoice::RequiredOrAuto) => {
            Some(AnthropicToolChoice {
                kind: "any".to_owned(),
                name: None,
            })
        }
        Some(ModelToolChoice::Function(name)) => Some(AnthropicToolChoice {
            kind: "tool".to_owned(),
            name: Some(name.clone()),
        }),
        None => None,
    })
}

fn anthropic_messages(
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<AnthropicMessage>> {
    let mut output = Vec::new();

    for message in messages {
        match message.role {
            ConversationRole::User => {
                append_blocks(&mut output, "user", user_blocks(model_id, message)?);
            }
            ConversationRole::Assistant => {
                if let Some(blocks) = message.provider_context.iter().find_map(|item| match item {
                    ProviderConversationItem::AnthropicAssistantMessage { blocks } => Some(blocks),
                    _ => None,
                }) {
                    output.push(AnthropicMessage {
                        role: "assistant".to_owned(),
                        content: blocks.iter().cloned().map(AnthropicBlock::from).collect(),
                    });
                    continue;
                }
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

    Ok(output)
}

fn user_blocks(model_id: &str, message: &ConversationMessage) -> ModelResult<Vec<AnthropicBlock>> {
    if message.content_parts.is_empty() {
        return Ok(vec![AnthropicBlock::Text {
            text: message.content.clone(),
        }]);
    }
    message
        .content_parts
        .iter()
        .map(|part| content_part(model_id, part))
        .collect()
}

fn content_part(model_id: &str, part: &ConversationContentPart) -> ModelResult<AnthropicBlock> {
    match part {
        ConversationContentPart::Text { text } => Ok(AnthropicBlock::Text { text: text.clone() }),
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => Ok(AnthropicBlock::Image {
            source: AnthropicImageSource {
                kind: "base64".to_owned(),
                media_type: mime_type.clone(),
                data: data_base64.clone(),
            },
        }),
        ConversationContentPart::Video { .. } => Err(ModelError::provider(
            PROVIDER,
            model_id,
            "Anthropic accepts text and image content parts only",
        )),
    }
}

fn append_blocks(messages: &mut Vec<AnthropicMessage>, role: &str, blocks: Vec<AnthropicBlock>) {
    if blocks.is_empty() {
        return;
    }
    if let Some(existing) = messages.last_mut()
        && existing.role == role
        && role == "user"
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
