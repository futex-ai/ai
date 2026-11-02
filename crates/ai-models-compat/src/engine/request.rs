//! Shared request mapping and structured-output instructions.

use ai_interface::{Effort, ModelRequest, ModelResult};
use ai_models_core::{ResolvedModelControls, ResolvedToolChoice};

use super::{
    controls::tool_choice,
    dialect::{Dialect, OutputLimit, StructuredOutput, ThinkingFormat, ToolChoiceWire},
    messages::{conversation_messages, system_message, validate_content_parts},
    request_types::{
        ChatCompletionsJsonSchema, ChatCompletionsRequest, ChatCompletionsResponseFormat,
        ChatCompletionsStreamOptions,
    },
    thinking::thinking_fields,
    tools::tool,
};

pub(crate) fn build_request(
    dialect: &Dialect,
    model_id: &str,
    request: &ModelRequest,
    controls: &ResolvedModelControls,
) -> ModelResult<ChatCompletionsRequest> {
    validate_content_parts(dialect, model_id, request)?;
    let effort = controls.effort;
    let (thinking, reasoning_effort) = thinking_fields(dialect, model_id, effort);
    let mut messages = Vec::new();
    let system_prompt = system_prompt(dialect, request);
    if !system_prompt.trim().is_empty() {
        messages.push(system_message(system_prompt));
    }
    messages.extend(conversation_messages(dialect, model_id, &request.messages)?);
    let omit_tools = matches!(dialect.tool_choice, ToolChoiceWire::OmitWhileThinking)
        && effort.is_enabled()
        && controls.tool_choice == Some(ResolvedToolChoice::None);
    let tools = if omit_tools {
        Vec::new()
    } else {
        request.tools.iter().map(tool).collect()
    };
    let max_output = controls.max_output_tokens;
    let deferred = controls.deferred;
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools,
        tool_choice: tool_choice(dialect, !request.tools.is_empty(), controls),
        parallel_tool_calls: (dialect.parallel_tools && !request.tools.is_empty()).then_some(true),
        enable_thinking: matches!(dialect.thinking, ThinkingFormat::Flags)
            .then_some(effort.is_enabled()),
        preserve_thinking: matches!(dialect.thinking, ThinkingFormat::Flags)
            .then_some(effort.is_enabled()),
        stream: !deferred,
        deferred,
        stream_options: (!deferred).then_some(ChatCompletionsStreamOptions {
            include_usage: true,
        }),
        reasoning_split: matches!(dialect.thinking, ThinkingFormat::Adaptive).then_some(true),
        thinking,
        reasoning_effort,
        response_format: response_format(dialect, model_id, effort, request),
        temperature: controls.temperature,
        top_p: controls.top_p,
        max_tokens: matches!(dialect.output_limit, OutputLimit::MaxTokens)
            .then_some(max_output)
            .flatten(),
        max_completion_tokens: matches!(dialect.output_limit, OutputLimit::MaxCompletionTokens)
            .then_some(max_output)
            .flatten(),
        stop: controls.stop_sequences.clone(),
    })
}

fn system_prompt(dialect: &Dialect, request: &ModelRequest) -> String {
    if matches!(dialect.structured_output, StructuredOutput::Prompt)
        && let Some(schema) = request.response_schema.as_ref()
    {
        return format!(
            "{}\n\nReturn only raw JSON that validates against the following JSON Schema. Do not use Markdown fences or include additional prose.\nSchema name: {}\nJSON Schema:\n{}",
            request.system_prompt, schema.name, schema.schema
        );
    }
    let Some(schema) = request.response_schema.as_ref().filter(|_| {
        matches!(
            dialect.structured_output,
            StructuredOutput::JsonObjectPrompt | StructuredOutput::JsonObjectWhenDisabled(_)
        )
    }) else {
        return request.system_prompt.clone();
    };
    format!(
        "{}\n\nReturn raw JSON only. Do not use Markdown fences or additional prose. \
The response must be a JSON object matching schema `{}`.\nJSON Schema:\n{}",
        request.system_prompt, schema.name, schema.schema
    )
}

fn response_format(
    dialect: &Dialect,
    model_id: &str,
    thinking: Effort,
    request: &ModelRequest,
) -> Option<ChatCompletionsResponseFormat> {
    let schema = request.response_schema.as_ref()?;
    Some(match dialect.structured_output {
        StructuredOutput::JsonObjectPrompt => ChatCompletionsResponseFormat::JsonObject,
        StructuredOutput::JsonObjectWhenDisabled(models)
            if models.contains(&model_id) && !thinking.is_enabled() =>
        {
            ChatCompletionsResponseFormat::JsonObject
        }
        StructuredOutput::JsonObjectWhenDisabled(_) | StructuredOutput::Prompt => return None,
        StructuredOutput::JsonSchema => ChatCompletionsResponseFormat::JsonSchema {
            json_schema: ChatCompletionsJsonSchema {
                name: schema.name.clone(),
                schema: schema.schema.clone(),
                strict: false,
            },
        },
    })
}
