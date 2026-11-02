//! MiniMax endpoint, wire settings, and provider status codes.

use ai_interface::{ErrorKind, ModelError};
use ai_models_core::{ErrorContext, json_http_error_core};
use json_http::JsonHttpHeaders;
use serde_json::Value;

use crate::engine::{
    dialect::{
        Dialect, InputParts, OutputLimit, ReplayRule, StructuredOutput, ThinkingFormat,
        ToolChoiceWire, UsageFields,
    },
    response::BaseResponse,
};

use super::stream_quirks::normalizer;

pub(super) static DIALECT: Dialect = Dialect {
    provider: "minimax",
    request_id_headers: &["trace-id", "x-request-id", "request-id"],
    label: "MiniMax",
    base_url: "https://api.minimax.io",
    path: "/v1/chat/completions",
    usage: UsageFields::CachedAndReasoning,
    replay: ReplayRule::MiniMax,
    thinking: ThinkingFormat::Adaptive,
    output_limit: OutputLimit::MaxCompletionTokens,
    structured_output: StructuredOutput::Prompt,
    tool_choice: ToolChoiceWire::AutoWhenTools,
    input: InputParts::TextImageVideo,
    names: true,
    required_reasoning: false,
    transient_finish: None,
    http_error: Some(http_error),
    parallel_tools: false,
    deferred_timeout: None,
    clean_eof: true,
    normalizer: Some(normalizer),
    base_error: Some(base_response_error),
};

fn base_response_error(context: &ErrorContext, base: Option<&BaseResponse>) -> Option<ModelError> {
    let base = base?;
    let code = base.status_code.unwrap_or_default();
    if code == 0 {
        return None;
    }
    let status_message = base.status_msg.as_deref().unwrap_or("unknown error");
    let message = format!("base_resp status_code {code}: {status_message}");
    Some(match code {
        1002 | 1041 | 2045 | 2056 => context.model_error(ErrorKind::RateLimited, message),
        1000 | 1001 | 1013 | 1024 | 1033 => context.model_error(ErrorKind::Transient, message),
        1039 => context.model_error(ErrorKind::ContextLimitExceeded, message),
        1004 | 1008 | 2049 => context.model_error(ErrorKind::RejectedKey, message),
        2022 => context.model_error(ErrorKind::UnknownModel, message),
        1027 => context.model_error(ErrorKind::InvalidOutput, message),
        _ => context.model_error(ErrorKind::InvalidRequest, message),
    })
}

fn http_error(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> ModelError {
    let mut core = json_http_error_core(context, status, headers, body);
    if let Some(base) = body.get("base_resp")
        && let Some(code) = base.get("status_code").and_then(Value::as_i64)
    {
        if status == 400 && code == 1039 {
            core.kind = ErrorKind::ContextLimitExceeded;
        }
        if let Some(message) = base.get("status_msg").and_then(Value::as_str) {
            core.message = context.redact(&format!(
                "HTTP {status}: base_resp status_code {code}: {message}"
            ));
        }
    }
    ModelError::new(core)
}
