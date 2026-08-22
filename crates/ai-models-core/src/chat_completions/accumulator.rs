//! Stateful ingestion of chat-completions SSE data payloads.

use super::{
    deltas::primary_deltas,
    state::ChatCompletionsState,
    types::{
        ChatCompletionsResponse, ChatCompletionsStreamChunk, ChatCompletionsStreamError,
        ChatCompletionsStreamStatus, ChatCompletionsStreamUpdate,
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
        match self.push_data_with_deltas(data)? {
            ChatCompletionsStreamUpdate::Chunk { deltas: _ } => {
                Ok(ChatCompletionsStreamStatus::Chunk)
            }
            ChatCompletionsStreamUpdate::Done => Ok(ChatCompletionsStreamStatus::Done),
        }
    }

    /// Parses one SSE `data` payload and returns primary-choice text fragments.
    pub fn push_data_with_deltas(
        &mut self,
        data: &str,
    ) -> Result<ChatCompletionsStreamUpdate, ChatCompletionsStreamError> {
        if self.done {
            return Err(ChatCompletionsStreamError::EventAfterDone);
        }
        if data == "[DONE]" {
            self.done = true;
            return Ok(ChatCompletionsStreamUpdate::Done);
        }
        let value = match serde_json::from_str::<serde_json::Value>(data) {
            Ok(value) => value,
            Err(source) => return Err(ChatCompletionsStreamError::DeserializeChunk { source }),
        };
        if let Some(error) = value.get("error").filter(|error| !error.is_null()) {
            return Err(ChatCompletionsStreamError::ProviderEvent {
                error: error.clone(),
            });
        }
        let chunk = match serde_json::from_value::<ChatCompletionsStreamChunk>(value) {
            Ok(chunk) => chunk,
            Err(source) => return Err(ChatCompletionsStreamError::DeserializeChunk { source }),
        };
        let deltas = primary_deltas(&chunk);
        self.state.push_chunk(chunk)?;
        Ok(ChatCompletionsStreamUpdate::Chunk { deltas })
    }

    /// Validates terminal state and returns a buffered response-shaped value.
    pub fn finish(self) -> Result<ChatCompletionsResponse, ChatCompletionsStreamError> {
        if !self.done {
            return Err(ChatCompletionsStreamError::MissingDone);
        }
        self.state.finish()
    }
}
