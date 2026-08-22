//! MiniMax model-specific stream normalization.

use std::collections::BTreeMap;

use ai_interface::MiniMaxReasoningDetail;
use serde_json::Value;
use thiserror::Error;

use crate::MINIMAX_M3;

#[derive(Debug, Error)]
pub(super) enum MiniMaxStreamError {
    #[error("[ai_models_minimax/stream] invalid streamed chunk JSON: {source}")]
    DeserializeChunk {
        #[source]
        source: serde_json::Error,
    },
}

pub(super) enum NormalizedEvent {
    Done,
    Chunk(Value),
}

pub(super) struct MiniMaxNormalizer {
    content: BTreeMap<u64, String>,
    content_mode: ContentMode,
    reasoning_details: BTreeMap<u64, Vec<MiniMaxReasoningDetail>>,
}

impl MiniMaxNormalizer {
    pub(super) fn new(provider_model_id: &str) -> Self {
        let content_mode = if provider_model_id == MINIMAX_M3 {
            ContentMode::Incremental
        } else {
            ContentMode::Cumulative
        };
        Self {
            content: BTreeMap::new(),
            content_mode,
            reasoning_details: BTreeMap::new(),
        }
    }

    pub(super) fn normalize(
        &mut self,
        data: &str,
    ) -> std::result::Result<NormalizedEvent, MiniMaxStreamError> {
        if data == "[DONE]" {
            return Ok(NormalizedEvent::Done);
        }
        let mut body = match serde_json::from_str::<Value>(data) {
            Ok(body) => body,
            Err(source) => return Err(MiniMaxStreamError::DeserializeChunk { source }),
        };
        let Some(choices) = body.get_mut("choices").and_then(Value::as_array_mut) else {
            return Ok(NormalizedEvent::Chunk(body));
        };
        for (position, choice) in choices.iter_mut().enumerate() {
            let choice_index = choice
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(position as u64);
            let Some(delta) = choice.get_mut("delta").and_then(Value::as_object_mut) else {
                continue;
            };
            if self.content_mode == ContentMode::Cumulative
                && let Some(current) = delta.get("content").and_then(Value::as_str)
            {
                let current = current.to_owned();
                delta.remove("content");
                self.content.insert(choice_index, current);
            }
            let Some(details) = delta.remove("reasoning_details") else {
                continue;
            };
            if details.is_null() {
                continue;
            }
            let current = match serde_json::from_value::<Vec<MiniMaxReasoningDetail>>(details) {
                Ok(current) => current,
                Err(source) => return Err(MiniMaxStreamError::DeserializeChunk { source }),
            };
            if !current.is_empty() {
                self.reasoning_details.insert(choice_index, current);
            }
        }
        Ok(NormalizedEvent::Chunk(body))
    }

    pub(super) fn restore_snapshots(
        self,
        body: &mut Value,
    ) -> std::result::Result<Option<String>, serde_json::Error> {
        let terminal_assistant_text = (self.content_mode == ContentMode::Cumulative)
            .then(|| self.content.get(&0).cloned())
            .flatten();
        let Some(choices) = body.get_mut("choices").and_then(Value::as_array_mut) else {
            return Ok(terminal_assistant_text);
        };
        for (position, choice) in choices.iter_mut().enumerate() {
            let choice_index = choice
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(position as u64);
            let Some(message) = choice.get_mut("message").and_then(Value::as_object_mut) else {
                continue;
            };
            if self.content_mode == ContentMode::Cumulative
                && let Some(content) = self.content.get(&choice_index)
            {
                message.insert("content".to_owned(), Value::String(content.clone()));
            }
            if let Some(details) = self.reasoning_details.get(&choice_index) {
                message.insert(
                    "reasoning_details".to_owned(),
                    serde_json::to_value(details)?,
                );
            }
        }
        Ok(terminal_assistant_text)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ContentMode {
    Cumulative,
    Incremental,
}
