//! OpenAI Responses function-call finish classification.

use super::types::ResponsesOutputItem;

pub(super) fn has_function_calls(output: &[ResponsesOutputItem]) -> bool {
    output
        .iter()
        .any(|item| matches!(item, ResponsesOutputItem::FunctionCall { .. }))
}
