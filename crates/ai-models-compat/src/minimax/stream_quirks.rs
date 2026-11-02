//! MiniMax normalization hooks for the common stream loop.

use ai_interface::{ErrorKind, ModelResult};
use ai_models_core::{ErrorContext, classify_stream_error};

use super::{
    dialect::DIALECT,
    stream_normalizer::{MiniMaxNormalizer, MiniMaxStreamError, NormalizedEvent},
};
use crate::{
    ChatCompletionsResponse,
    engine::{response::stream_base_response_error, stream::StreamNormalizer},
};

pub(super) fn normalizer(model_id: &str) -> Box<dyn StreamNormalizer> {
    Box::new(MiniMaxNormalizer::new(model_id))
}

impl StreamNormalizer for MiniMaxNormalizer {
    fn normalize_data(
        &mut self,
        data: &str,
        context: &ErrorContext,
        progress: u64,
    ) -> ModelResult<String> {
        let normalized = match self.normalize(data) {
            Ok(normalized) => normalized,
            Err(source) => {
                return Err(classify_stream_error(context, progress, &source));
            }
        };
        match normalized {
            NormalizedEvent::Done => Ok("[DONE]".to_owned()),
            NormalizedEvent::Chunk(body) => {
                let error = match stream_base_response_error(&DIALECT, context, &body) {
                    Ok(error) => error,
                    Err(source) => {
                        return Err(classify_stream_error(
                            context,
                            progress,
                            &MiniMaxStreamError::DeserializeChunk { source },
                        ));
                    }
                };
                if let Some(error) = error {
                    if progress == 0 {
                        return Err(error);
                    }
                    return Err(context.model_error(ErrorKind::Interrupted, error.to_string()));
                }
                Ok(body.to_string())
            }
        }
    }

    fn restore(self: Box<Self>, body: &mut ChatCompletionsResponse) -> Option<String> {
        (*self).restore_snapshots(body)
    }
}
