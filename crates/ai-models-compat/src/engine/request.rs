//! Shared request controls and JSON-output instruction mapping.

use ai_interface::{ModelControl, ModelError, ModelRequest, ModelResult, ModelToolChoice};
use ai_models_core::ThinkingLevel;

use super::{
    dialect::Dialect,
    messages::message,
    request_types::{
        ChatCompletionsMessage, ChatCompletionsNamedFunction, ChatCompletionsNamedToolChoice,
        ChatCompletionsRequest, ChatCompletionsResponseFormat, ChatCompletionsStreamOptions,
        ChatCompletionsThinking, ChatCompletionsTool, ChatCompletionsToolChoice,
    },
    tools::tool,
};

pub(crate) fn build_request(
    dialect: &Dialect,
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    validate_content_parts(dialect, model_id, request)?;
    let (thinking, reasoning_effort) = thinking_fields(dialect, model_id, thinking_level)?;
    validate_controls(dialect, model_id, thinking_level, request)?;
    let mut messages = Vec::new();
    let system_prompt = system_prompt(request);
    if !system_prompt.trim().is_empty() {
        messages.push(ChatCompletionsMessage {
            role: "system".to_owned(),
            content: system_prompt,
            name: None,
            tool_call_id: None,
            reasoning_content: None,
            tool_calls: Vec::new(),
        });
    }
    messages.extend(request.messages.iter().map(message));
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: mapped_tools(thinking_level, request),
        tool_choice: tool_choice(thinking_level, request),
        stream: true,
        stream_options: ChatCompletionsStreamOptions {
            include_usage: true,
        },
        thinking,
        reasoning_effort,
        response_format: request
            .response_schema
            .as_ref()
            .map(|_| ChatCompletionsResponseFormat {
                kind: "json_object".to_owned(),
            }),
        temperature: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.temperature)
            .flatten(),
        top_p: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.top_p)
            .flatten(),
        max_tokens: request.controls.generation.max_output_tokens,
        stop: request.controls.generation.stop_sequences.clone(),
    })
}

fn validate_controls(
    dialect: &Dialect,
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<()> {
    if thinking_level.is_enabled()
        && matches!(
            request.controls.generation.tool_choice,
            Some(ModelToolChoice::Required | ModelToolChoice::Function(_))
        )
    {
        return Err(ModelError::unsupported_control(
            dialect.provider,
            model_id,
            ModelControl::ToolChoice,
        ));
    }
    Ok(())
}

fn mapped_tools(thinking_level: ThinkingLevel, request: &ModelRequest) -> Vec<ChatCompletionsTool> {
    if thinking_level.is_enabled()
        && request.controls.generation.tool_choice == Some(ModelToolChoice::None)
    {
        return Vec::new();
    }
    request.tools.iter().map(tool).collect()
}

fn tool_choice(
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> Option<ChatCompletionsToolChoice> {
    if thinking_level.is_enabled() {
        return None;
    }
    match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::None) => Some(ChatCompletionsToolChoice::Mode("none".to_owned())),
        Some(ModelToolChoice::Auto) => Some(ChatCompletionsToolChoice::Mode("auto".to_owned())),
        Some(ModelToolChoice::Required) => {
            Some(ChatCompletionsToolChoice::Mode("required".to_owned()))
        }
        Some(ModelToolChoice::RequiredOrAuto) => {
            Some(ChatCompletionsToolChoice::Mode("required".to_owned()))
        }
        Some(ModelToolChoice::Function(name)) => Some(ChatCompletionsToolChoice::Function(
            ChatCompletionsNamedToolChoice {
                kind: "function".to_owned(),
                function: ChatCompletionsNamedFunction { name: name.clone() },
            },
        )),
        None => None,
    }
}

fn system_prompt(request: &ModelRequest) -> String {
    let Some(schema) = request.response_schema.as_ref() else {
        return request.system_prompt.clone();
    };
    format!(
        "{}\n\nReturn raw JSON only. Do not use Markdown fences or additional prose. \
The response must be a JSON object matching schema `{}`.\nJSON Schema:\n{}",
        request.system_prompt, schema.name, schema.schema
    )
}

fn validate_content_parts(
    dialect: &Dialect,
    model_id: &str,
    request: &ModelRequest,
) -> ModelResult<()> {
    if request
        .messages
        .iter()
        .any(|message| !message.content_parts.is_empty())
    {
        return Err(ModelError::provider(
            dialect.provider,
            model_id,
            "DeepSeek accepts plain text messages only",
        ));
    }
    Ok(())
}

fn thinking_fields(
    dialect: &Dialect,
    model_id: &str,
    thinking_level: ThinkingLevel,
) -> ModelResult<(ChatCompletionsThinking, Option<String>)> {
    let (kind, reasoning_effort) = match thinking_level {
        ThinkingLevel::Disabled => ("disabled", None),
        ThinkingLevel::High => ("enabled", Some("high".to_owned())),
        ThinkingLevel::Max => ("enabled", Some("max".to_owned())),
        ThinkingLevel::Low | ThinkingLevel::Medium | ThinkingLevel::ExtraHigh => {
            return Err(ModelError::provider(
                dialect.provider,
                model_id,
                "unsupported DeepSeek thinking level",
            ));
        }
    };
    Ok((
        ChatCompletionsThinking {
            kind: kind.to_owned(),
        },
        reasoning_effort,
    ))
}
