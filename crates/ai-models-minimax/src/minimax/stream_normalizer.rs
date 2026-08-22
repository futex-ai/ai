//! MiniMax cumulative stream snapshot normalization.

use std::collections::BTreeMap;

use ai_interface::MiniMaxReasoningDetail;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum MiniMaxStreamError {
    #[error("[ai_models_minimax/stream] invalid streamed chunk JSON: {source}")]
    DeserializeChunk {
        #[source]
        source: serde_json::Error,
    },
    #[error("[ai_models_minimax/stream] choice {choice_index} replaced cumulative content")]
    ReplacedContent { choice_index: u64 },
    #[error(
        "[ai_models_minimax/stream] choice {choice_index} reasoning detail {detail_index} replaced cumulative text"
    )]
    ReplacedReasoning {
        choice_index: u64,
        detail_index: u32,
    },
}

pub(super) enum NormalizedEvent {
    Done,
    Chunk(Value),
}

#[derive(Default)]
pub(super) struct MiniMaxNormalizer {
    content: BTreeMap<u64, String>,
    reasoning_details: BTreeMap<u64, Vec<MiniMaxReasoningDetail>>,
}

impl MiniMaxNormalizer {
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
            if let Some(current) = delta.get("content").and_then(Value::as_str) {
                let previous = self.content.get(&choice_index).map_or("", String::as_str);
                let Some(fragment) = current.strip_prefix(previous) else {
                    return Err(MiniMaxStreamError::ReplacedContent { choice_index });
                };
                let current = current.to_owned();
                delta.insert("content".to_owned(), Value::String(fragment.to_owned()));
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
            self.validate_reasoning(choice_index, &current)?;
            if !current.is_empty() {
                self.reasoning_details.insert(choice_index, current);
            }
        }
        Ok(NormalizedEvent::Chunk(body))
    }

    fn validate_reasoning(
        &self,
        choice_index: u64,
        current: &[MiniMaxReasoningDetail],
    ) -> std::result::Result<(), MiniMaxStreamError> {
        let Some(previous) = self.reasoning_details.get(&choice_index) else {
            return Ok(());
        };
        for (position, detail) in current.iter().enumerate() {
            let detail_index = detail.index.unwrap_or(position as u32);
            let previous = previous.iter().enumerate().find(|(position, previous)| {
                previous.index.unwrap_or(*position as u32) == detail_index
            });
            let Some((_, previous)) = previous else {
                continue;
            };
            if let (Some(previous), Some(current)) = (&previous.text, &detail.text)
                && !current.starts_with(previous)
            {
                return Err(MiniMaxStreamError::ReplacedReasoning {
                    choice_index,
                    detail_index,
                });
            }
        }
        Ok(())
    }

    pub(super) fn restore_reasoning_details(
        self,
        body: &mut Value,
    ) -> std::result::Result<(), serde_json::Error> {
        let Some(choices) = body.get_mut("choices").and_then(Value::as_array_mut) else {
            return Ok(());
        };
        for (position, choice) in choices.iter_mut().enumerate() {
            let choice_index = choice
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(position as u64);
            let Some(details) = self.reasoning_details.get(&choice_index) else {
                continue;
            };
            let Some(message) = choice.get_mut("message").and_then(Value::as_object_mut) else {
                continue;
            };
            message.insert(
                "reasoning_details".to_owned(),
                serde_json::to_value(details)?,
            );
        }
        Ok(())
    }
}
