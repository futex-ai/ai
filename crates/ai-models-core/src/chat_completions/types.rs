//! Typed stream chunks, accumulated response values, and validation errors.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Buffered response shape reconstructed from chat-completions deltas.
pub struct ChatCompletionsResponse {
    /// Choices ordered by their provider-assigned index.
    pub choices: Vec<ChatCompletionsChoice>,
    /// Final provider usage report.
    pub usage: ChatCompletionsUsage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One accumulated chat-completions choice.
pub struct ChatCompletionsChoice {
    /// Provider-assigned choice index.
    pub index: u64,
    /// Complete assistant message reconstructed from deltas.
    pub message: ChatCompletionsMessage,
    /// Terminal provider finish reason.
    pub finish_reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Complete assistant message reconstructed from streamed deltas.
pub struct ChatCompletionsMessage {
    /// Concatenated visible text, preserving absence separately from empty text.
    pub content: Option<String>,
    /// Concatenated provider reasoning, when supplied.
    pub reasoning_content: Option<String>,
    /// Complete tool calls ordered by their provider-assigned index.
    pub tool_calls: Vec<ChatCompletionsToolCall>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One complete streamed tool call.
pub struct ChatCompletionsToolCall {
    /// Provider-issued tool-call identifier.
    pub id: String,
    /// Function name and concatenated JSON argument text.
    pub function: ChatCompletionsToolFunction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Function payload reconstructed from tool-call deltas.
pub struct ChatCompletionsToolFunction {
    /// Provider-selected function name.
    pub name: String,
    /// Concatenated raw JSON argument text.
    pub arguments: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Superset of usage fields emitted by compatible chat-completions providers.
pub struct ChatCompletionsUsage {
    /// Total input tokens before provider-specific cache normalization.
    #[serde(default)]
    pub prompt_tokens: u64,
    /// Total generated tokens before reasoning-token normalization.
    #[serde(default)]
    pub completion_tokens: u64,
    /// Provider-reported total tokens, when supplied.
    #[serde(default)]
    pub total_tokens: Option<u64>,
    /// DeepSeek-style cache-hit input tokens.
    #[serde(default)]
    pub prompt_cache_hit_tokens: u64,
    /// DeepSeek-style cache-miss input tokens, when supplied.
    #[serde(default)]
    pub prompt_cache_miss_tokens: Option<u64>,
    /// OpenAI-style prompt-token details.
    #[serde(default)]
    pub prompt_tokens_details: ChatCompletionsPromptTokenDetails,
    /// OpenAI-style completion-token details.
    #[serde(default)]
    pub completion_tokens_details: ChatCompletionsCompletionTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Provider prompt-token detail fields retained by the accumulator.
pub struct ChatCompletionsPromptTokenDetails {
    /// Input tokens served from a provider cache.
    #[serde(default)]
    pub cached_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Provider completion-token detail fields retained by the accumulator.
pub struct ChatCompletionsCompletionTokenDetails {
    /// Generated tokens attributed to private reasoning.
    #[serde(default)]
    pub reasoning_tokens: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Outcome of ingesting one SSE data payload.
pub enum ChatCompletionsStreamStatus {
    /// A JSON chunk was merged and more events are expected.
    Chunk,
    /// The terminal `[DONE]` sentinel was received.
    Done,
}

#[derive(Debug, Error)]
/// Validation errors returned while accumulating chat-completions streams.
pub enum ChatCompletionsStreamError {
    /// A non-terminal data payload was not valid chunk JSON.
    #[error("[ai_models_core/chat_completions] invalid streamed chunk JSON: {source}")]
    DeserializeChunk {
        /// Underlying JSON decoding failure.
        source: serde_json::Error,
    },
    /// Data arrived after the terminal sentinel.
    #[error("[ai_models_core/chat_completions] received data after [DONE]")]
    EventAfterDone,
    /// Stream EOF occurred without the terminal sentinel.
    #[error("[ai_models_core/chat_completions] stream ended without [DONE]")]
    MissingDone,
    /// No choice deltas were received.
    #[error("[ai_models_core/chat_completions] stream contained no choices")]
    MissingChoices,
    /// A choice did not receive a terminal finish reason.
    #[error(
        "[ai_models_core/chat_completions] choice {choice_index} had no terminal finish reason"
    )]
    MissingFinishReason {
        /// Provider-assigned choice index.
        choice_index: u64,
    },
    /// A choice reported two different terminal finish reasons.
    #[error(
        "[ai_models_core/chat_completions] choice {choice_index} changed finish reason from `{existing}` to `{received}`"
    )]
    ConflictingFinishReason {
        /// Provider-assigned choice index.
        choice_index: u64,
        /// First terminal finish reason.
        existing: String,
        /// Conflicting later finish reason.
        received: String,
    },
    /// No final provider usage object was received.
    #[error("[ai_models_core/chat_completions] stream contained no final usage")]
    MissingUsage,
    /// A tool call completed without an identifier.
    #[error("[ai_models_core/chat_completions] choice {choice_index} tool {tool_index} had no id")]
    MissingToolCallId {
        /// Provider-assigned choice index.
        choice_index: u64,
        /// Provider-assigned tool-call index.
        tool_index: u64,
    },
    /// A tool call completed without a function name.
    #[error(
        "[ai_models_core/chat_completions] choice {choice_index} tool {tool_index} had no function name"
    )]
    MissingToolFunctionName {
        /// Provider-assigned choice index.
        choice_index: u64,
        /// Provider-assigned tool-call index.
        tool_index: u64,
    },
    /// A tool-call index was associated with conflicting identifiers.
    #[error(
        "[ai_models_core/chat_completions] choice {choice_index} tool {tool_index} changed id from `{existing}` to `{received}`"
    )]
    ConflictingToolCallId {
        /// Provider-assigned choice index.
        choice_index: u64,
        /// Provider-assigned tool-call index.
        tool_index: u64,
        /// First provider tool-call identifier.
        existing: String,
        /// Conflicting later identifier.
        received: String,
    },
    /// A tool-call index was associated with conflicting function names.
    #[error(
        "[ai_models_core/chat_completions] choice {choice_index} tool {tool_index} changed function from `{existing}` to `{received}`"
    )]
    ConflictingToolFunctionName {
        /// Provider-assigned choice index.
        choice_index: u64,
        /// Provider-assigned tool-call index.
        tool_index: u64,
        /// First provider function name.
        existing: String,
        /// Conflicting later function name.
        received: String,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsStreamChunk {
    #[serde(default)]
    pub(super) choices: Vec<ChatCompletionsChoiceDelta>,
    #[serde(default)]
    pub(super) usage: Option<ChatCompletionsUsage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsChoiceDelta {
    #[serde(default)]
    pub(super) index: u64,
    #[serde(default)]
    pub(super) delta: ChatCompletionsMessageDelta,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ChatCompletionsMessageDelta {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
    #[serde(default)]
    pub(super) tool_calls: Vec<ChatCompletionsToolCallDelta>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsToolCallDelta {
    #[serde(default)]
    pub(super) index: u64,
    #[serde(default)]
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) function: ChatCompletionsToolFunctionDelta,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ChatCompletionsToolFunctionDelta {
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) arguments: Option<String>,
}
