//! Shared request-to-QwenCloud Chat Completions mapping.

use ai_interface::{
    ConversationRole, ModelControl, ModelError, ModelRequest, ModelResult, ModelToolChoice,
    ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use crate::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS};

use super::request_messages::message;
use super::request_types::{
    ChatCompletionsContent, ChatCompletionsMessage, ChatCompletionsNamedFunction,
    ChatCompletionsNamedToolChoice, ChatCompletionsRequest, ChatCompletionsResponseFormat,
    ChatCompletionsTool, ChatCompletionsToolChoice, ChatCompletionsToolDefinition,
};

const PROVIDER: &str = "qwen";

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    validate_content_parts(model_id, request)?;
    let thinking_enabled = thinking_level.is_enabled();
    validate_controls(model_id, thinking_enabled, request)?;
    let mut messages = Vec::new();
    let system_prompt = system_prompt(request);
    if !system_prompt.trim().is_empty() {
        messages.push(ChatCompletionsMessage {
            role: "system".to_owned(),
            content: Some(ChatCompletionsContent::Text(system_prompt)),
            tool_call_id: None,
            reasoning_content: None,
            tool_calls: Vec::new(),
        });
    }
    for conversation_message in &request.messages {
        messages.push(message(model_id, conversation_message)?);
    }
    let has_tools = !request.tools.is_empty();
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: tool_choice(thinking_enabled, request, has_tools),
        parallel_tool_calls: has_tools.then_some(true),
        stream: false,
        enable_thinking: thinking_enabled,
        preserve_thinking: thinking_enabled,
        response_format: native_json_format(model_id, thinking_level, request),
        temperature: (!thinking_enabled)
            .then_some(request.controls.generation.temperature)
            .flatten(),
        top_p: (!thinking_enabled)
            .then_some(request.controls.generation.top_p)
            .flatten(),
        max_completion_tokens: request.controls.generation.max_output_tokens,
        stop: request.controls.generation.stop_sequences.clone(),
    })
}

fn validate_controls(
    model_id: &str,
    thinking_enabled: bool,
    request: &ModelRequest,
) -> ModelResult<()> {
    let unsupported = match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::Required | ModelToolChoice::Function(_)) if thinking_enabled => {
            Some(ModelControl::ToolChoice)
        }
        Some(
            ModelToolChoice::None
            | ModelToolChoice::Auto
            | ModelToolChoice::Required
            | ModelToolChoice::RequiredOrAuto
            | ModelToolChoice::Function(_),
        )
        | None => None,
    };
    match unsupported {
        Some(control) => Err(ModelError::unsupported_control(PROVIDER, model_id, control)),
        None => Ok(()),
    }
}

fn tool_choice(
    thinking_enabled: bool,
    request: &ModelRequest,
    has_tools: bool,
) -> Option<ChatCompletionsToolChoice> {
    match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::None) => Some(ChatCompletionsToolChoice::Mode("none".to_owned())),
        Some(ModelToolChoice::Auto) => Some(ChatCompletionsToolChoice::Mode("auto".to_owned())),
        Some(ModelToolChoice::RequiredOrAuto) if thinking_enabled => {
            Some(ChatCompletionsToolChoice::Mode("auto".to_owned()))
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
        Some(ModelToolChoice::Required) => {
            Some(ChatCompletionsToolChoice::Mode("required".to_owned()))
        }
        None if has_tools => Some(ChatCompletionsToolChoice::Mode("auto".to_owned())),
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

fn native_json_format(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> Option<ChatCompletionsResponseFormat> {
    let supports_native_json = matches!(model_id, QWEN_3_7_PLUS | QWEN_3_7_FLASH)
        && thinking_level == ThinkingLevel::Disabled;
    (request.response_schema.is_some() && supports_native_json).then(|| {
        ChatCompletionsResponseFormat {
            kind: "json_object".to_owned(),
        }
    })
}

fn validate_content_parts(model_id: &str, request: &ModelRequest) -> ModelResult<()> {
    for message in &request.messages {
        if message.content_parts.is_empty() {
            continue;
        }
        if model_id == QWEN_3_7_MAX {
            return Err(ModelError::provider(
                PROVIDER,
                model_id,
                "qwen3.7-max accepts plain text messages only",
            ));
        }
        if message.role != ConversationRole::User {
            return Err(ModelError::provider(
                PROVIDER,
                model_id,
                "Qwen accepts typed content parts only on user messages",
            ));
        }
    }
    Ok(())
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
