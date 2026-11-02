//! One Chat Completions engine with provider-specific dialects.

#![warn(unreachable_pub)]

mod chat_completions;
mod engine;

pub mod deepseek;

pub use chat_completions::{
    ChatCompletionsAccumulator, ChatCompletionsChoice, ChatCompletionsCompletionTokenDetails,
    ChatCompletionsDelta, ChatCompletionsMessage, ChatCompletionsPromptTokenDetails,
    ChatCompletionsResponse, ChatCompletionsStreamError, ChatCompletionsStreamStatus,
    ChatCompletionsStreamUpdate, ChatCompletionsToolCall, ChatCompletionsToolFunction,
    ChatCompletionsUsage,
};
pub use engine::errors::classify_chat_completions_provider_error;
