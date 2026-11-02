//! MiniMax endpoint, wire settings, and provider status codes.

use super::{MINIMAX_M3, stream_quirks::normalizer};
use crate::engine::{
    dialect::{
        Dialect, InputParts, OutputLimit, ReplayRule, Sampling, StructuredOutput, ThinkingFormat,
        ToolChoiceRule,
    },
    response::BaseResponse,
};
use ai_interface::ModelError;

pub(super) static DIALECT: Dialect = Dialect {
    provider: "minimax",
    label: "MiniMax",
    endpoint: "https://api.minimax.io/v1/chat/completions",
    replay: ReplayRule::MiniMax,
    thinking: ThinkingFormat::Adaptive,
    output_limit: OutputLimit::MaxCompletionTokens,
    structured_output: StructuredOutput::Prompt,
    sampling: Sampling::Any,
    tool_choice: ToolChoiceRule::RequiredFor(MINIMAX_M3),
    input: InputParts::TextImageVideo,
    names: true,
    required_reasoning: false,
    transient_finish: None,
    http_error: None,
    parallel_tools: false,
    deferred_timeout: None,
    clean_eof: true,
    normalizer: Some(normalizer),
    base_error: Some(base_response_error),
};

fn base_response_error(model_id: &str, base: Option<&BaseResponse>) -> Option<ModelError> {
    let base = base?;
    let code = base.status_code.unwrap_or_default();
    if code == 0 {
        return None;
    }
    let status_message = base.status_msg.as_deref().unwrap_or("unknown error");
    let message = format!("base_resp status_code {code}: {status_message}");
    Some(match code {
        1002 | 1041 | 2045 | 2056 => ModelError::rate_limited("minimax", model_id, message),
        1000 | 1001 | 1013 | 1024 | 1033 => {
            ModelError::transient_provider("minimax", model_id, message)
        }
        1039 => ModelError::context_limit_exceeded("minimax", model_id, message),
        _ => ModelError::provider("minimax", model_id, message),
    })
}
