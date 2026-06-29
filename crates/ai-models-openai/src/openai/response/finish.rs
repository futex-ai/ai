//! OpenAI Responses finish reason normalization.

use ai_interface::FinishReason;

use super::types::{ResponsesContentPart, ResponsesOutputItem, ResponsesResponse};

pub(super) fn finish_reason(response: &ResponsesResponse, has_tool_calls: bool) -> FinishReason {
    match response.status.as_deref() {
        Some("incomplete") => return incomplete_finish_reason(response),
        Some("failed" | "cancelled") => {
            return FinishReason::Other(
                response
                    .status
                    .clone()
                    .unwrap_or_else(|| "failed".to_owned()),
            );
        }
        Some("completed") | None => {}
        Some(raw) => return FinishReason::Other(raw.to_owned()),
    }
    if has_tool_calls {
        return FinishReason::ToolCalls;
    }
    if has_refusal(&response.output) {
        return FinishReason::Filtered;
    }
    FinishReason::Stop
}

fn incomplete_finish_reason(response: &ResponsesResponse) -> FinishReason {
    match response
        .incomplete_details
        .as_ref()
        .and_then(|details| details.reason.as_deref())
    {
        Some("max_output_tokens") => FinishReason::Truncated,
        Some("content_filter") => FinishReason::Filtered,
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("incomplete".to_owned()),
    }
}

fn has_refusal(output: &[ResponsesOutputItem]) -> bool {
    output.iter().any(|item| match item {
        ResponsesOutputItem::Message { content } => content
            .iter()
            .any(|part| matches!(part, ResponsesContentPart::Refusal { .. })),
        ResponsesOutputItem::Reasoning { .. }
        | ResponsesOutputItem::FunctionCall { .. }
        | ResponsesOutputItem::Other => false,
    })
}
