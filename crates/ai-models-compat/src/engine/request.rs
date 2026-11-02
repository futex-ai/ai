//! Shared request mapping and structured-output instructions.

use ai_interface::{ModelCompletionMode, ModelRequest, ModelResult, ModelToolChoice};
use ai_models_core::ThinkingLevel;

use super::{
    controls::{tool_choice, validate_controls},
    dialect::{Dialect, OutputLimit, Sampling, StructuredOutput, ThinkingFormat, ToolChoiceRule},
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
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    validate_content_parts(dialect, model_id, request)?;
    let (thinking, reasoning_effort) = thinking_fields(dialect, model_id, thinking_level)?;
    validate_controls(dialect, model_id, thinking_level, request)?;
    let mut messages = Vec::new();
    let system_prompt = system_prompt(dialect, request);
    if !system_prompt.trim().is_empty() {
        messages.push(system_message(system_prompt));
    }
    messages.extend(conversation_messages(dialect, model_id, &request.messages)?);
    let omit_tools = matches!(dialect.tool_choice, ToolChoiceRule::OmitWhileThinking)
        && thinking_level.is_enabled()
        && request.controls.generation.tool_choice == Some(ModelToolChoice::None);
    let tools = if omit_tools {
        Vec::new()
    } else {
        request.tools.iter().map(tool).collect()
    };
    let sampling = match dialect.sampling {
        Sampling::WithoutThinking => !thinking_level.is_enabled(),
        Sampling::Fixed => false,
        Sampling::Any => true,
    };
    let max_output = request.controls.generation.max_output_tokens;
    let deferred = dialect.deferred_timeout.is_some()
        && request.controls.execution.completion_mode != ModelCompletionMode::Synchronous;
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools,
        tool_choice: tool_choice(dialect, model_id, thinking_level, request),
        parallel_tool_calls: (dialect.parallel_tools && !request.tools.is_empty()).then_some(true),
        enable_thinking: matches!(dialect.thinking, ThinkingFormat::Flags)
            .then_some(thinking_level.is_enabled()),
        preserve_thinking: matches!(dialect.thinking, ThinkingFormat::Flags)
            .then_some(thinking_level.is_enabled()),
        stream: !deferred,
        deferred,
        stream_options: (!deferred).then_some(ChatCompletionsStreamOptions {
            include_usage: true,
        }),
        reasoning_split: matches!(dialect.thinking, ThinkingFormat::Adaptive).then_some(true),
        thinking,
        reasoning_effort,
        response_format: response_format(dialect, model_id, thinking_level, request),
        temperature: sampling
            .then_some(request.controls.generation.temperature)
            .flatten(),
        top_p: sampling
            .then_some(request.controls.generation.top_p)
            .flatten(),
        max_tokens: matches!(dialect.output_limit, OutputLimit::MaxTokens)
            .then_some(max_output)
            .flatten(),
        max_completion_tokens: matches!(dialect.output_limit, OutputLimit::MaxCompletionTokens)
            .then_some(max_output)
            .flatten(),
        stop: request.controls.generation.stop_sequences.clone(),
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
    thinking: ThinkingLevel,
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
