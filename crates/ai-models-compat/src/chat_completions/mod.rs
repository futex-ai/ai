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

#[cfg(test)]
#[path = "_tests_/chat_completions_compatibility_tests.rs"]
mod chat_completions_compatibility_tests;

#[cfg(test)]
#[path = "_tests_/chat_completions_event_tests.rs"]
mod chat_completions_event_tests;

#[cfg(test)]
#[path = "_tests_/chat_completions_tests.rs"]
mod chat_completions_tests;
