//! One Chat Completions engine with provider-specific dialects.

#![warn(unreachable_pub)]

mod chat_completions;
mod engine;

pub mod deepseek;
pub mod kimi;
pub mod minimax;
pub mod qwen;
pub mod xai;

pub use chat_completions::{
    ChatCompletionsAccumulator, ChatCompletionsChoice, ChatCompletionsCompletionTokenDetails,
    ChatCompletionsDelta, ChatCompletionsMessage, ChatCompletionsPromptTokenDetails,
    ChatCompletionsResponse, ChatCompletionsStreamError, ChatCompletionsStreamStatus,
    ChatCompletionsStreamUpdate, ChatCompletionsToolCall, ChatCompletionsToolFunction,
    ChatCompletionsUsage,
};
pub use engine::errors::classify_chat_completions_provider_error;

#[cfg(test)]
#[path = "_tests_/stream_support.rs"]
pub(crate) mod stream_test_support;

#[cfg(test)]
#[path = "_tests_/part_support.rs"]
mod part_test_support;
