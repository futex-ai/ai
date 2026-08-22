//! Typed Anthropic stream events and accumulator failures.

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum AnthropicEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicMessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u64,
        content_block: AnthropicContentBlockStart,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u64,
        delta: AnthropicContentBlockDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u64 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        #[serde(default)]
        delta: AnthropicMessageDelta,
        #[serde(default)]
        usage: AnthropicUsageDelta,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error {
        error: AnthropicProviderErrorPayload,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub(super) struct AnthropicMessageStart {
    #[serde(default)]
    pub(super) usage: AnthropicUsage,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct AnthropicUsage {
    #[serde(default)]
    pub(super) input_tokens: u32,
    #[serde(default)]
    pub(super) output_tokens: u32,
    #[serde(default)]
    pub(super) cache_read_input_tokens: u32,
    #[serde(default)]
    pub(super) cache_creation_input_tokens: u32,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct AnthropicUsageDelta {
    pub(super) input_tokens: Option<u32>,
    pub(super) output_tokens: Option<u32>,
    pub(super) cache_read_input_tokens: Option<u32>,
    pub(super) cache_creation_input_tokens: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct AnthropicMessageDelta {
    pub(super) stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum AnthropicContentBlockStart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
    #[serde(other)]
    Ignored,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum AnthropicContentBlockDelta {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
    #[serde(rename = "signature_delta")]
    Signature { signature: String },
    #[serde(other)]
    Ignored,
}

#[derive(Debug, Deserialize)]
pub(super) struct AnthropicProviderErrorPayload {
    #[serde(rename = "type")]
    kind: String,
    pub(super) message: String,
}

impl AnthropicProviderErrorPayload {
    pub(super) fn kind(&self) -> AnthropicProviderErrorKind {
        AnthropicProviderErrorKind::from_raw(&self.kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AnthropicProviderErrorKind {
    RateLimit,
    Overloaded,
    Api,
    RequestTooLarge,
    InvalidRequest,
    Authentication,
    Permission,
    NotFound,
    Unrecognized,
}

impl AnthropicProviderErrorKind {
    fn from_raw(raw: &str) -> Self {
        match raw {
            "rate_limit_error" => Self::RateLimit,
            "overloaded_error" => Self::Overloaded,
            "api_error" => Self::Api,
            "request_too_large" => Self::RequestTooLarge,
            "invalid_request_error" => Self::InvalidRequest,
            "authentication_error" => Self::Authentication,
            "permission_error" => Self::Permission,
            "not_found_error" => Self::NotFound,
            _ => Self::Unrecognized,
        }
    }
}

#[derive(Debug)]
pub(super) enum AnthropicAccumulation {
    Continue { delta: Option<AnthropicStreamDelta> },
    Complete(Value),
}

#[derive(Debug)]
pub(super) enum AnthropicStreamDelta {
    AssistantText { delta: String, starts_block: bool },
    ReasoningText { delta: String },
}

#[derive(Debug, Error)]
pub(super) enum AnthropicStreamError {
    #[error("[ai_models_anthropic/stream] invalid event JSON: {source}")]
    DeserializeEvent { source: serde_json::Error },
    #[error("[ai_models_anthropic/stream] received an event after message_stop")]
    EventAfterMessageStop,
    #[error("[ai_models_anthropic/stream] received a duplicate message_start")]
    DuplicateMessageStart,
    #[error("[ai_models_anthropic/stream] received {event} before message_start")]
    EventBeforeMessageStart { event: &'static str },
    #[error("[ai_models_anthropic/stream] content block index {index} started twice")]
    DuplicateContentBlock { index: u64 },
    #[error("[ai_models_anthropic/stream] unknown content block index {index}")]
    UnknownContentBlock { index: u64 },
    #[error("[ai_models_anthropic/stream] content block index {index} received data after stop")]
    ContentBlockAfterStop { index: u64 },
    #[error(
        "[ai_models_anthropic/stream] {delta_kind} delta did not match {block_kind} block index {index}"
    )]
    MismatchedBlockDelta {
        index: u64,
        block_kind: &'static str,
        delta_kind: &'static str,
    },
    #[error("[ai_models_anthropic/stream] content block index {index} stopped twice")]
    DuplicateContentBlockStop { index: u64 },
    #[error(
        "[ai_models_anthropic/stream] tool block index {index} had invalid input JSON: {source}"
    )]
    InvalidToolInput {
        index: u64,
        source: serde_json::Error,
    },
    #[error("[ai_models_anthropic/stream] message_stop arrived with open content blocks")]
    OpenContentBlocks,
    #[error("[ai_models_anthropic/stream] provider event {kind:?}: {message}")]
    ProviderEvent {
        kind: AnthropicProviderErrorKind,
        message: String,
    },
    #[error("[ai_models_anthropic/stream] response body ended before message_stop")]
    UnexpectedEof,
}
