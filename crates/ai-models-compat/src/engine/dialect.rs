//! Dialect settings and invocation identity shared by the engine stages.

use std::time::Duration;

use ai_interface::{Effort, ModelError};
use serde_json::Value;

use super::{response::BaseResponse, stream::StreamNormalizer};

type NormalizerFactory = fn(&str) -> Box<dyn StreamNormalizer>;
type BaseErrorClassifier = fn(&str, Option<&BaseResponse>) -> Option<ModelError>;

pub(crate) struct Dialect {
    pub(crate) provider: &'static str,
    pub(crate) label: &'static str,
    pub(crate) endpoint: &'static str,
    pub(crate) replay: ReplayRule,
    pub(crate) usage: UsageFields,
    pub(crate) thinking: ThinkingFormat,
    pub(crate) output_limit: OutputLimit,
    pub(crate) structured_output: StructuredOutput,
    pub(crate) tool_choice: ToolChoiceWire,
    pub(crate) input: InputParts,
    pub(crate) names: bool,
    pub(crate) required_reasoning: bool,
    pub(crate) transient_finish: Option<(&'static str, &'static str)>,
    pub(crate) http_error: Option<fn(&str, u16, &Value) -> ModelError>,
    pub(crate) parallel_tools: bool,
    pub(crate) clean_eof: bool,
    pub(crate) deferred_timeout: Option<Duration>,
    pub(crate) normalizer: Option<NormalizerFactory>,
    pub(crate) base_error: Option<BaseErrorClassifier>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ReplayRule {
    DeepSeek,
    Kimi,
    Qwen,
    MiniMax,
    Xai,
}

#[derive(Clone, Copy)]
pub(crate) enum UsageFields {
    CacheHitMiss,
    DirectCached,
    CachedInput,
    CachedAndReasoning,
    BoundedCachedAndReasoning,
}

#[derive(Clone, Copy)]
pub(crate) enum ThinkingFormat {
    ToggleEffort,
    Effort,
    Flags,
    Adaptive,
    GrokEffort { fixed_prefix: &'static str },
}

#[derive(Clone, Copy)]
pub(crate) enum OutputLimit {
    MaxTokens,
    MaxCompletionTokens,
}

#[derive(Clone, Copy)]
pub(crate) enum StructuredOutput {
    JsonObjectPrompt,
    JsonSchema,
    JsonObjectWhenDisabled(&'static [&'static str]),
    Prompt,
}

#[derive(Clone, Copy)]
pub(crate) enum ToolChoiceWire {
    OmitWhileThinking,
    AutoWhenTools,
}

#[derive(Clone, Copy)]
pub(crate) enum InputParts {
    PlainText,
    TextImage,
    TextImageVideo,
    UserTextImage { plain_text_model: &'static str },
}

#[derive(Clone, Copy)]
pub(crate) struct CompletionContext<'a> {
    pub(crate) dialect: &'a Dialect,
    pub(crate) catalog_model_id: &'a str,
    pub(crate) provider_model_id: &'a str,
    pub(crate) thinking_level: Effort,
    pub(crate) synthetic_tool_call_scope: &'a str,
}
