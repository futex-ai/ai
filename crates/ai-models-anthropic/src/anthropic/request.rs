//! Anthropic messages request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, Effort, ModelError,
    ModelRequest, ModelResult, ProviderConversationItem, ToolDefinition,
};
use ai_models_core::{ResolvedModelControls, ResolvedToolChoice};

use super::request_types::{
    AnthropicBlock, AnthropicImageSource, AnthropicMessage, AnthropicOutputConfig,
    AnthropicRequest, AnthropicThinking, AnthropicTool, AnthropicToolChoice,
};

const PROVIDER: &str = "anthropic";

pub(super) fn build_request(
    model_id: &str,
    request: &ModelRequest,
    controls: &ResolvedModelControls,
) -> ModelResult<AnthropicRequest> {
    Ok(AnthropicRequest {
        stream: true,
        model: model_id.to_owned(),
        max_tokens: controls.max_output_tokens,
        system: system_prompt(request),
        messages: anthropic_messages(model_id, &request.messages)?,
        tools: request.tools.iter().map(tool).collect(),
        thinking: thinking(controls.effort),
        output_config: output_config(controls.effort),
        temperature: controls.temperature,
        top_p: controls.top_p,
        stop_sequences: controls.stop_sequences.clone(),
        tool_choice: tool_choice(controls.tool_choice.as_ref()),
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

fn tool_choice(choice: Option<&ResolvedToolChoice>) -> Option<AnthropicToolChoice> {
    match choice {
        Some(ResolvedToolChoice::None) => Some(AnthropicToolChoice {
            kind: "none".to_owned(),
            name: None,
        }),
        Some(ResolvedToolChoice::Auto) => Some(AnthropicToolChoice {
            kind: "auto".to_owned(),
            name: None,
        }),
        Some(ResolvedToolChoice::Required) => Some(AnthropicToolChoice {
            kind: "any".to_owned(),
            name: None,
        }),
        Some(ResolvedToolChoice::Function(name)) => Some(AnthropicToolChoice {
            kind: "tool".to_owned(),
            name: Some(name.clone()),
        }),
        None => None,
    }
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

fn thinking(thinking_level: Effort) -> Option<AnthropicThinking> {
    thinking_level.is_enabled().then(|| AnthropicThinking {
        kind: "adaptive".to_owned(),
        display: "omitted".to_owned(),
    })
}

fn output_config(thinking_level: Effort) -> Option<AnthropicOutputConfig> {
    effort(thinking_level).map(|effort| AnthropicOutputConfig {
        effort: effort.to_owned(),
    })
}

fn effort(thinking_level: Effort) -> Option<&'static str> {
    match thinking_level {
        Effort::Disabled => None,
        Effort::Low => Some("low"),
        Effort::Medium => Some("medium"),
        Effort::High => Some("high"),
        Effort::ExtraHigh => Some("xhigh"),
        Effort::Max => Some("max"),
    }
}
