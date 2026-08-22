//! OpenAI-compatible chat-completions stream accumulation.

mod accumulator;
mod state;
mod types;

pub use accumulator::ChatCompletionsAccumulator;
pub use types::{
    ChatCompletionsChoice, ChatCompletionsCompletionTokenDetails, ChatCompletionsMessage,
    ChatCompletionsPromptTokenDetails, ChatCompletionsResponse, ChatCompletionsStreamError,
    ChatCompletionsStreamStatus, ChatCompletionsToolCall, ChatCompletionsToolFunction,
    ChatCompletionsUsage,
};
