//! OpenAI-compatible chat-completions stream accumulation.

mod accumulator;
mod deltas;
mod state;
mod types;

pub use accumulator::ChatCompletionsAccumulator;
pub use types::{
    ChatCompletionsChoice, ChatCompletionsCompletionTokenDetails, ChatCompletionsDelta,
    ChatCompletionsMessage, ChatCompletionsPromptTokenDetails, ChatCompletionsResponse,
    ChatCompletionsStreamError, ChatCompletionsStreamStatus, ChatCompletionsStreamUpdate,
    ChatCompletionsToolCall, ChatCompletionsToolFunction, ChatCompletionsUsage,
};
