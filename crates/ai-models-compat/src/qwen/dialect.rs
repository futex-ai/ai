//! Qwen endpoint, wire settings, and input-length error quirk.

use super::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS};
use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, Sampling, StructuredOutput, ThinkingFormat,
    ToolChoiceRule,
};
use ai_interface::ModelError;
use ai_models_core::classify_json_http_error;
use serde_json::Value;

pub(super) static DIALECT: Dialect = Dialect {
    provider: "qwen",
    label: "Qwen",
    endpoint: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions",
    replay: ReplayRule::Qwen,
    thinking: ThinkingFormat::Flags,
    output_limit: OutputLimit::MaxCompletionTokens,
    structured_output: StructuredOutput::JsonObjectWhenDisabled(&[QWEN_3_7_PLUS, QWEN_3_7_FLASH]),
    sampling: Sampling::WithoutThinking,
    tool_choice: ToolChoiceRule::AutoWhileThinking,
    input: InputParts::UserTextImage {
        plain_text_model: QWEN_3_7_MAX,
    },
    names: false,
    parallel_tools: true,
    required_reasoning: true,
    transient_finish: None,
    http_error: Some(classify_qwen_http_error),
};

pub(super) fn classify_qwen_http_error(model_id: &str, status: u16, body: &Value) -> ModelError {
    if status == 400
        && let Some(message) = error_message(body)
        && message.starts_with("Range of input length should be [1,")
    {
        return ModelError::context_limit_exceeded(
            "qwen",
            model_id,
            format!("HTTP {status}: {message}"),
        );
    }
    classify_json_http_error("qwen", model_id, status, body)
}

fn error_message(body: &Value) -> Option<&str> {
    body.get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
}
