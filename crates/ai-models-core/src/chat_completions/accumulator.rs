//! Stateful ingestion of chat-completions SSE data payloads.

use super::{
    state::ChatCompletionsState,
    types::{
        ChatCompletionsResponse, ChatCompletionsStreamChunk, ChatCompletionsStreamError,
        ChatCompletionsStreamStatus,
    },
};

#[derive(Debug, Default)]
/// Pure accumulator for OpenAI-compatible chat-completions SSE data payloads.
pub struct ChatCompletionsAccumulator {
    state: ChatCompletionsState,
    done: bool,
}

impl ChatCompletionsAccumulator {
    /// Creates an empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses and merges one SSE `data` payload.
    pub fn push_data(
        &mut self,
        data: &str,
    ) -> Result<ChatCompletionsStreamStatus, ChatCompletionsStreamError> {
        if self.done {
            return Err(ChatCompletionsStreamError::EventAfterDone);
        }
        if data == "[DONE]" {
            self.done = true;
            return Ok(ChatCompletionsStreamStatus::Done);
        }
        let chunk = match serde_json::from_str::<ChatCompletionsStreamChunk>(data) {
            Ok(chunk) => chunk,
            Err(source) => return Err(ChatCompletionsStreamError::DeserializeChunk { source }),
        };
        self.state.push_chunk(chunk)?;
        Ok(ChatCompletionsStreamStatus::Chunk)
    }

    /// Validates terminal state and returns a buffered response-shaped value.
    pub fn finish(self) -> Result<ChatCompletionsResponse, ChatCompletionsStreamError> {
        if !self.done {
            return Err(ChatCompletionsStreamError::MissingDone);
        }
        self.state.finish()
    }
}
