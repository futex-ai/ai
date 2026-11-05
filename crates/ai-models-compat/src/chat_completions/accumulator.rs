//! Stateful ingestion of chat-completions SSE data payloads.

use super::chunk::ChatCompletionsStreamChunk;

use super::{
    deltas::primary_deltas,
    state::ChatCompletionsState,
    types::{
        ChatCompletionsResponse, ChatCompletionsStreamError, ChatCompletionsStreamStatus,
        ChatCompletionsStreamUpdate,
    },
};

#[derive(Debug, Default)]
/// Pure accumulator for OpenAI-compatible chat-completions SSE data payloads.
pub struct ChatCompletionsAccumulator {
    state: ChatCompletionsState,
    done: bool,
    model_stream: bool,
}

impl ChatCompletionsAccumulator {
    /// Creates an empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn for_model_stream() -> Self {
        Self {
            model_stream: true,
            ..Self::default()
        }
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
        match self.push_data_with_chunk(data)? {
            Some(chunk) => Ok(ChatCompletionsStreamUpdate::Chunk {
                deltas: primary_deltas(&chunk),
            }),
            None => Ok(ChatCompletionsStreamUpdate::Done),
        }
    }

    pub(crate) fn push_data_with_chunk(
        &mut self,
        data: &str,
    ) -> Result<Option<ChatCompletionsStreamChunk>, ChatCompletionsStreamError> {
        if self.done {
            return Err(ChatCompletionsStreamError::EventAfterDone);
        }
        if data == "[DONE]" {
            self.done = true;
            return Ok(None);
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
        let mut chunk = match serde_json::from_value::<ChatCompletionsStreamChunk>(value) {
            Ok(chunk) => chunk,
            Err(source) => return Err(ChatCompletionsStreamError::DeserializeChunk { source }),
        };
        if self.model_stream {
            chunk.choices.retain(|choice| choice.index == 0);
        }
        self.state.push_chunk(chunk.clone(), self.model_stream)?;
        Ok(Some(chunk))
    }

    /// Validates terminal state and returns a buffered response-shaped value.
    pub fn finish(self) -> Result<ChatCompletionsResponse, ChatCompletionsStreamError> {
        self.finish_model_stream(false)
    }

    pub(crate) fn finish_model_stream(
        self,
        explicit_done: bool,
    ) -> Result<ChatCompletionsResponse, ChatCompletionsStreamError> {
        if !self.done {
            return Err(ChatCompletionsStreamError::MissingDone);
        }
        self.state.finish(self.model_stream && explicit_done)
    }
}
