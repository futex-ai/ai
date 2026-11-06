//! Qwen endpoint, wire settings, and input-length error quirk.

use ai_interface::{ErrorKind, ModelError};
use ai_models_core::{ErrorContext, classify_json_http_error};
use json_http::JsonHttpHeaders;
use serde_json::Value;

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, StructuredOutput, ThinkingFormat, ToolChoiceWire,
    UsageFields,
};

use super::catalog::{QWEN_3_7_FLASH_ID, QWEN_3_7_MAX_ID, QWEN_3_7_PLUS_ID};

pub(super) static DIALECT: Dialect = Dialect {
    provider: "qwen",
    request_id_headers: &["x-request-id", "request-id"],
    label: "Qwen",
    base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode",
    path: "/v1/chat/completions",
    usage: UsageFields::CachedInput,
    replay: ReplayRule::Qwen,
    thinking: ThinkingFormat::Flags,
    output_limit: OutputLimit::MaxCompletionTokens,
    structured_output: StructuredOutput::JsonObjectWhenDisabled(&[
        QWEN_3_7_PLUS_ID,
        QWEN_3_7_FLASH_ID,
    ]),
    tool_choice: ToolChoiceWire::AutoWhenTools {
        omit_without_tools: false,
    },
    input: InputParts::UserTextImage {
        plain_text_model: QWEN_3_7_MAX_ID,
    },
    deferred_timeout: None,
    clean_eof: false,
    normalizer: None,
    base_error: None,
    names: false,
    parallel_tools: true,
    required_reasoning: true,
    transient_finish: None,
    http_error: Some(classify_qwen_http_error),
};

pub(super) fn classify_qwen_http_error(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> ModelError {
    if status == 400
        && let Some(message) = error_message(body)
        && message.starts_with("Range of input length should be [1,")
    {
        return context.for_response(headers).model_error(
            ErrorKind::ContextLimitExceeded,
            format!("HTTP {status}: {message}"),
        );
    }
    classify_json_http_error(context, status, headers, body)
}

fn error_message(body: &Value) -> Option<&str> {
    body.get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
}
