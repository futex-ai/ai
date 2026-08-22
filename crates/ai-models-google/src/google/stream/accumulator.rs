//! Google stream fragment merging and terminal response construction.

use serde_json::{Map, Value, json};

use super::{
    part::AccumulatedPart,
    types::{
        GooglePromptFeedback, GoogleStreamChunk, GoogleStreamDelta, GoogleStreamError,
        GoogleStreamUpdate, GoogleUsageMetadata,
    },
};

#[derive(Debug, Default)]
pub(super) struct GoogleStreamAccumulator {
    candidate_seen: bool,
    parts: Vec<AccumulatedPart>,
    finish_reason: Option<String>,
    prompt_feedback: Option<GooglePromptFeedback>,
    usage: Option<GoogleUsageMetadata>,
    assistant_parts_emitted: usize,
}

impl GoogleStreamAccumulator {
    pub(super) fn push_data(
        &mut self,
        data: &str,
    ) -> Result<GoogleStreamUpdate, GoogleStreamError> {
        let chunk = match serde_json::from_str::<GoogleStreamChunk>(data) {
            Ok(chunk) => chunk,
            Err(source) => return Err(GoogleStreamError::DeserializeChunk { source }),
        };
        if let Some(error) = chunk.error {
            return Ok(GoogleStreamUpdate::ProviderError(error));
        }
        if let Some(prompt_feedback) = chunk.prompt_feedback {
            self.prompt_feedback = Some(prompt_feedback);
        }
        if let Some(usage) = chunk.usage_metadata {
            self.usage = Some(usage);
        }
        let mut deltas = Vec::new();
        if let Some(candidate) = chunk.candidates.into_iter().next() {
            self.candidate_seen = true;
            if let Some(content) = candidate.content {
                for (index, part) in content.parts.into_iter().enumerate() {
                    if let Some(delta) = AccumulatedPart::append(&mut self.parts, part, index == 0)
                    {
                        deltas.push(self.normalize_delta(delta));
                    }
                }
            }
            if let Some(finish_reason) = candidate.finish_reason {
                self.finish_reason = Some(finish_reason);
            }
        }
        Ok(GoogleStreamUpdate::Continue { deltas })
    }

    fn normalize_delta(&mut self, delta: GoogleStreamDelta) -> GoogleStreamDelta {
        match delta {
            GoogleStreamDelta::AssistantText {
                mut delta,
                starts_part: true,
            } => {
                if self.assistant_parts_emitted > 0 {
                    delta.insert(0, '\n');
                }
                self.assistant_parts_emitted = self.assistant_parts_emitted.saturating_add(1);
                GoogleStreamDelta::AssistantText {
                    delta,
                    starts_part: true,
                }
            }
            other => other,
        }
    }

    pub(super) fn finish_body(&self) -> Result<Value, GoogleStreamError> {
        if self.finish_reason.is_none() && !self.prompt_was_blocked() {
            return Err(GoogleStreamError::MissingTerminal);
        }
        let mut body = Map::new();
        body.insert("candidates".to_owned(), self.candidates_body());
        if let Some(prompt_feedback) = &self.prompt_feedback {
            body.insert(
                "promptFeedback".to_owned(),
                json!({"blockReason": prompt_feedback.block_reason}),
            );
        }
        if let Some(usage) = &self.usage {
            body.insert("usageMetadata".to_owned(), usage_body(usage));
        }
        Ok(Value::Object(body))
    }

    fn prompt_was_blocked(&self) -> bool {
        self.prompt_feedback
            .as_ref()
            .and_then(|feedback| feedback.block_reason.as_ref())
            .is_some()
    }

    fn candidates_body(&self) -> Value {
        if !self.candidate_seen {
            return Value::Array(Vec::new());
        }
        let parts = self
            .parts
            .iter()
            .map(AccumulatedPart::body)
            .collect::<Vec<_>>();
        let mut candidate = Map::new();
        candidate.insert("content".to_owned(), json!({"parts": parts}));
        if let Some(finish_reason) = &self.finish_reason {
            candidate.insert(
                "finishReason".to_owned(),
                Value::String(finish_reason.clone()),
            );
        }
        Value::Array(vec![Value::Object(candidate)])
    }
}

fn usage_body(usage: &GoogleUsageMetadata) -> Value {
    json!({
        "promptTokenCount": usage.prompt_token_count,
        "candidatesTokenCount": usage.candidates_token_count,
        "totalTokenCount": usage.total_token_count,
        "cachedContentTokenCount": usage.cached_content_token_count,
        "thoughtsTokenCount": usage.thoughts_token_count
    })
}
