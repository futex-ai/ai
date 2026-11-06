//! Kimi endpoint and wire settings.

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, StructuredOutput, ThinkingFormat, ToolChoiceWire,
    UsageFields,
};

pub(super) static DIALECT: Dialect = Dialect {
    provider: "kimi",
    request_id_headers: &["x-request-id", "request-id"],
    label: "Kimi",
    base_url: "https://api.moonshot.ai",
    path: "/v1/chat/completions",
    usage: UsageFields::DirectCached,
    replay: ReplayRule::Kimi,
    thinking: ThinkingFormat::Effort,
    output_limit: OutputLimit::MaxCompletionTokens,
    structured_output: StructuredOutput::JsonSchema,
    tool_choice: ToolChoiceWire::AutoWhenTools {
        omit_without_tools: false,
    },
    input: InputParts::TextImage,
    deferred_timeout: None,
    clean_eof: false,
    normalizer: None,
    base_error: None,
    names: true,
    parallel_tools: false,
    http_error: None,
    transient_finish: None,
    required_reasoning: false,
};
