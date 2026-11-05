//! Native deferred content retained before finish filtering.

use ai_interface::{ModelResponse, ModelResult, StructuredOutputSchema};
use serde_json::Value;

use super::{
    super::dialect::CompletionContext,
    legacy,
    parse::{decode_response, map_response},
};

/// Native content emitted after the deferred provider result arrives.
pub(crate) struct DeferredContent {
    /// Exposed reasoning that the xAI replay mapper does not retain.
    pub(crate) reasoning: Option<String>,
    /// Every native modern or normalized legacy call, before finish filtering.
    pub(crate) calls: Vec<DeferredTool>,
}

/// Raw tool content for a finite deferred stream.
pub(crate) struct DeferredTool {
    /// Provider or deterministic legacy identifier.
    pub(crate) id: String,
    /// Requested function name.
    pub(crate) name: String,
    /// Native argument text, including incomplete non-dispatchable arguments.
    pub(crate) arguments: String,
    /// Runtime operation identity, when a legacy call requires one.
    pub(crate) operation_id: Option<String>,
}

pub(crate) fn parse_deferred(
    context: CompletionContext<'_>,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<(ModelResponse, DeferredContent)> {
    let reasoning = body["choices"][0]["message"]["reasoning_content"]
        .as_str()
        .map(str::to_owned);
    let (parsed, usage) = decode_response(context, body)?;
    let calls = parsed
        .choices
        .first()
        .map(|choice| {
            let mut calls = choice.message.parsed_tool_calls.clone();
            legacy::normalize(context, &mut calls, choice.message.function_call.clone());
            calls
                .unwrap_or_default()
                .into_iter()
                .map(|call| DeferredTool {
                    id: call.id,
                    name: call.function.name,
                    arguments: call.function.arguments,
                    operation_id: call.operation_id,
                })
                .collect()
        })
        .unwrap_or_default();
    let response = map_response(context, parsed, usage, schema)?;
    Ok((response, DeferredContent { reasoning, calls }))
}
