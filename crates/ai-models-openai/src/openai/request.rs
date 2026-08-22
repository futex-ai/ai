//! OpenAI Responses request mapping.

use ai_interface::{
    ModelRequest, ModelResult, ModelToolChoice, StructuredOutputSchema, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::request_input::input_items;
use super::request_types::{
    ResponsesNamedToolChoice, ResponsesReasoning, ResponsesRequest, ResponsesText,
    ResponsesTextFormat, ResponsesTool, ResponsesToolChoice,
};

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ResponsesRequest> {
    let tools = request.tools.iter().map(tool).collect::<Vec<_>>();
    Ok(ResponsesRequest {
        stream: true,
        model: model_id.to_owned(),
        instructions: request.nonblank_system_prompt().map(str::to_owned),
        input: input_items(model_id, &request.messages)?,
        store: false,
        include: include_items(thinking_level),
        tool_choice: tool_choice(request, !tools.is_empty()),
        tools,
        text: request.response_schema.as_ref().map(text_format),
        reasoning: reasoning(thinking_level),
        temperature: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.temperature)
            .flatten(),
        top_p: (!thinking_level.is_enabled())
            .then_some(request.controls.generation.top_p)
            .flatten(),
        max_output_tokens: request.controls.generation.max_output_tokens,
    })
}

fn tool_choice(request: &ModelRequest, has_tools: bool) -> Option<ResponsesToolChoice> {
    match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::None) => Some(ResponsesToolChoice::Mode("none".to_owned())),
        Some(ModelToolChoice::Auto) => Some(ResponsesToolChoice::Mode("auto".to_owned())),
        Some(ModelToolChoice::Required | ModelToolChoice::RequiredOrAuto) => {
            Some(ResponsesToolChoice::Mode("required".to_owned()))
        }
        Some(ModelToolChoice::Function(name)) => {
            Some(ResponsesToolChoice::Function(ResponsesNamedToolChoice {
                kind: "function".to_owned(),
                name: name.clone(),
            }))
        }
        None if has_tools => Some(ResponsesToolChoice::Mode("auto".to_owned())),
        None => None,
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
            strict: false,
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
        ThinkingLevel::ExtraHigh => "xhigh",
        ThinkingLevel::Max => "max",
    };
    Some(ResponsesReasoning {
        effort: effort.to_owned(),
    })
}

fn include_items(thinking_level: ThinkingLevel) -> Vec<String> {
    thinking_level
        .is_enabled()
        .then(|| "reasoning.encrypted_content".to_owned())
        .into_iter()
        .collect()
}
