//! MiniMax normalization hooks for the common stream loop.

use ai_interface::{ModelError, ModelResult};
use ai_models_core::classify_stream_error;
use serde_json::Value;

use super::{
    dialect::DIALECT,
    stream_normalizer::{MiniMaxNormalizer, MiniMaxStreamError, NormalizedEvent},
};
use crate::engine::{response::stream_base_response_error, stream::StreamNormalizer};

pub(super) fn normalizer(model_id: &str) -> Box<dyn StreamNormalizer> {
    Box::new(MiniMaxNormalizer::new(model_id))
}

impl StreamNormalizer for MiniMaxNormalizer {
    fn normalize_data(&mut self, data: &str, model_id: &str, progress: u64) -> ModelResult<String> {
        let normalized = match self.normalize(data) {
            Ok(normalized) => normalized,
            Err(source) => {
                return Err(classify_stream_error(
                    "minimax", model_id, progress, &source,
                ));
            }
        };
        match normalized {
            NormalizedEvent::Done => Ok("[DONE]".to_owned()),
            NormalizedEvent::Chunk(body) => {
                let error = match stream_base_response_error(&DIALECT, model_id, &body) {
                    Ok(error) => error,
                    Err(source) => {
                        return Err(classify_stream_error(
                            "minimax",
                            model_id,
                            progress,
                            &MiniMaxStreamError::DeserializeChunk { source },
                        ));
                    }
                };
                if let Some(error) = error {
                    if progress == 0 {
                        return Err(error);
                    }
                    return Err(ModelError::interrupted(
                        "minimax",
                        model_id,
                        error.to_string(),
                    ));
                }
                Ok(body.to_string())
            }
        }
    }

    fn restore(self: Box<Self>, body: &mut Value) -> Result<Option<String>, serde_json::Error> {
        (*self).restore_snapshots(body)
    }
}
