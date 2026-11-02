//! Direct mapping from typed stream state without terminal JSON serialization.

use ai_interface::{ErrorKind, ModelError, ModelResponse, ModelResult, StructuredOutputSchema};

use crate::{ChatCompletionsResponse, ChatCompletionsToolFunction};

use super::{
    super::dialect::{CompletionContext, ReplayRule},
    parse::map_response,
    stream_usage, types,
};

pub(crate) fn parse_accumulated(
    context: CompletionContext<'_>,
    accumulated: ChatCompletionsResponse,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let usage = match stream_usage::normalize(context.dialect.usage, accumulated.usage) {
        Ok(usage) => usage,
        Err(source) => {
            return Err(match context.dialect.replay {
                ReplayRule::Xai => ModelError::with_source(
                    context.error.core(
                        ErrorKind::InvalidOutput,
                        format!("invalid xAI response: {source}"),
                    ),
                    source,
                ),
                _ => ModelError::with_source(
                    context
                        .error
                        .core(ErrorKind::InvalidOutput, "invalid provider response"),
                    source,
                ),
            });
        }
    };
    let parsed = types::ChatCompletionsResponse {
        choices: accumulated
            .choices
            .into_iter()
            .map(|choice| types::ChatCompletionsChoice {
                finish_reason: Some(choice.finish_reason),
                message: types::ChatCompletionsAssistantMessage {
                    content: choice.message.content,
                    reasoning_content: choice.message.reasoning_content,
                    reasoning_details: choice.message.reasoning_details,
                    tool_calls: None,
                    parsed_tool_calls: Some(
                        choice
                            .message
                            .tool_calls
                            .into_iter()
                            .map(|call| types::ChatCompletionsToolCall {
                                id: call.id,
                                function: function(call.function),
                                operation_id: None,
                            })
                            .collect(),
                    ),
                    function_call: choice
                        .message
                        .function_call
                        .filter(|_| context.dialect.replay == ReplayRule::Xai)
                        .map(function),
                },
            })
            .collect(),
        usage: None,
        base_resp: None,
    };
    map_response(context, parsed, usage, schema)
}

fn function(function: ChatCompletionsToolFunction) -> types::ChatCompletionsToolFunction {
    types::ChatCompletionsToolFunction {
        name: function.name,
        arguments: function.arguments,
    }
}
