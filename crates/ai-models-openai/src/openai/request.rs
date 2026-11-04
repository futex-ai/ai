//! OpenAI Responses request mapping.

use ai_interface::{Effort, ModelRequest, ModelResult, StructuredOutputSchema, ToolDefinition};
use ai_models_core::{ErrorContext, ResolvedModelControls, ResolvedToolChoice};

use super::request_input::input_items;
use super::request_types::{
    ResponsesNamedToolChoice, ResponsesReasoning, ResponsesRequest, ResponsesText,
    ResponsesTextFormat, ResponsesTool, ResponsesToolChoice,
};

pub(super) fn build_request(
    context: &ErrorContext,
    model_id: &str,
    request: &ModelRequest,
    controls: &ResolvedModelControls,
) -> ModelResult<ResponsesRequest> {
    let tools = request.tools.iter().map(tool).collect::<Vec<_>>();
    Ok(ResponsesRequest {
        stream: true,
        model: model_id.to_owned(),
        instructions: request.nonblank_system_prompt().map(str::to_owned),
        input: input_items(context, model_id, &request.messages)?,
        store: false,
        include: include_items(controls.effort),
        tool_choice: tool_choice(controls.tool_choice.as_ref(), !tools.is_empty()),
        tools,
        text: request.response_schema.as_ref().map(text_format),
        reasoning: reasoning(controls.effort),
        temperature: controls.temperature,
        top_p: controls.top_p,
        max_output_tokens: controls.max_output_tokens,
    })
}

fn tool_choice(
    choice: Option<&ResolvedToolChoice>,
    has_tools: bool,
) -> Option<ResponsesToolChoice> {
    match choice {
        Some(ResolvedToolChoice::None) => Some(ResponsesToolChoice::Mode("none".to_owned())),
        Some(ResolvedToolChoice::Auto) => Some(ResponsesToolChoice::Mode("auto".to_owned())),
        Some(ResolvedToolChoice::Required) => {
            Some(ResponsesToolChoice::Mode("required".to_owned()))
        }
        Some(ResolvedToolChoice::Function(name)) => {
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

fn reasoning(thinking_level: Option<Effort>) -> Option<ResponsesReasoning> {
    let effort = match thinking_level? {
        Effort::Disabled => "none",
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::ExtraHigh => "xhigh",
        Effort::Max => "max",
    };
    Some(ResponsesReasoning {
        effort: effort.to_owned(),
    })
}

fn include_items(thinking_level: Option<Effort>) -> Vec<String> {
    thinking_level
        .is_some_and(Effort::is_enabled)
        .then(|| "reasoning.encrypted_content".to_owned())
        .into_iter()
        .collect()
}
