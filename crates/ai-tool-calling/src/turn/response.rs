//! Model response execution and validation helpers.

use std::time::Instant;

use ai_interface::{
    FinishReason, ModelCallLogEntry, ModelCallLogResult, ModelRequest, ModelResponse,
    ProviderConversationItem,
};

use crate::{Error, Result, ToolCallingRuntime};

use super::failures::provider_error;

pub(super) async fn complete_model_request(
    runtime: &ToolCallingRuntime,
    request: ModelRequest,
) -> Result<ModelResponse> {
    let started_at = Instant::now();
    let response = match runtime.model.complete(&request).await {
        Ok(response) => response,
        Err(error) => {
            runtime.logger.log_model_call(&ModelCallLogEntry {
                request: model_request_for_logging(request),
                result: ModelCallLogResult::Error {
                    message: error.to_string(),
                    debug: format!("{error:?}"),
                },
                latency_ms: started_at.elapsed().as_millis(),
            })?;
            return Err(Error::Model(error));
        }
    };
    runtime.logger.log_model_call(&ModelCallLogEntry {
        request: model_request_for_logging(request),
        result: ModelCallLogResult::Success {
            response: Box::new(model_response_for_logging(&response)),
        },
        latency_ms: started_at.elapsed().as_millis(),
    })?;
    Ok(response)
}

fn model_request_for_logging(mut request: ModelRequest) -> ModelRequest {
    for message in &mut request.messages {
        remove_private_replay_context(&mut message.provider_context);
    }
    request
}

fn model_response_for_logging(response: &ModelResponse) -> ModelResponse {
    let mut response = response.clone();
    remove_private_replay_context(&mut response.provider_context);
    response
}

fn remove_private_replay_context(context: &mut Vec<ProviderConversationItem>) {
    context.retain(|item| {
        !matches!(
            item,
            ProviderConversationItem::KimiAssistantMessage { .. }
                | ProviderConversationItem::MiniMaxAssistant { .. }
        )
    });
}

pub(super) fn validate_response_contract(response: &ModelResponse) -> Result<()> {
    if response.finish_reason != FinishReason::ToolCalls && !response.tool_calls.is_empty() {
        return Err(provider_error(
            response,
            "model returned tool calls without a tool-call finish reason",
        ));
    }
    match &response.finish_reason {
        FinishReason::Stop | FinishReason::Truncated => Ok(()),
        FinishReason::Filtered => Err(provider_error(
            response,
            "model response was filtered by the provider",
        )),
        FinishReason::Other(raw) => Err(provider_error(response, raw.clone())),
        FinishReason::ToolCalls if response.tool_calls.is_empty() => Err(provider_error(
            response,
            "model reported tool calls without any tool call payloads",
        )),
        FinishReason::ToolCalls => Ok(()),
    }
}
