//! Anthropic event-state validation and delta accumulation.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{
    block::ContentBlockState,
    types::{
        AnthropicAccumulation, AnthropicEvent, AnthropicStreamError, AnthropicUsage,
        AnthropicUsageDelta,
    },
};

#[derive(Debug, Default)]
pub(super) struct AnthropicStreamAccumulator {
    message_started: bool,
    message_stopped: bool,
    blocks: BTreeMap<u64, ContentBlockState>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

impl AnthropicStreamAccumulator {
    pub(super) fn push_data(
        &mut self,
        data: &str,
    ) -> Result<AnthropicAccumulation, AnthropicStreamError> {
        if self.message_stopped {
            return Err(AnthropicStreamError::EventAfterMessageStop);
        }
        let event = match serde_json::from_str::<AnthropicEvent>(data) {
            Ok(event) => event,
            Err(source) => return Err(AnthropicStreamError::DeserializeEvent { source }),
        };
        match event {
            AnthropicEvent::MessageStart { message } => {
                if self.message_started {
                    return Err(AnthropicStreamError::DuplicateMessageStart);
                }
                self.message_started = true;
                self.usage = Some(message.usage);
            }
            AnthropicEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                self.require_started("content_block_start")?;
                if self
                    .blocks
                    .insert(index, ContentBlockState::new(content_block))
                    .is_some()
                {
                    return Err(AnthropicStreamError::DuplicateContentBlock { index });
                }
            }
            AnthropicEvent::ContentBlockDelta { index, delta } => {
                self.require_started("content_block_delta")?;
                let Some(block) = self.blocks.get_mut(&index) else {
                    return Err(AnthropicStreamError::UnknownContentBlock { index });
                };
                block.push_delta(index, delta)?;
            }
            AnthropicEvent::ContentBlockStop { index } => {
                self.require_started("content_block_stop")?;
                let Some(block) = self.blocks.get_mut(&index) else {
                    return Err(AnthropicStreamError::UnknownContentBlock { index });
                };
                block.stop(index)?;
            }
            AnthropicEvent::MessageDelta { delta, usage } => {
                self.require_started("message_delta")?;
                if let Some(stop_reason) = delta.stop_reason {
                    self.stop_reason = Some(stop_reason);
                }
                if let Some(current) = self.usage.as_mut() {
                    current.merge(usage);
                }
            }
            AnthropicEvent::MessageStop => {
                self.require_started("message_stop")?;
                let body = self.complete_body()?;
                self.message_stopped = true;
                return Ok(AnthropicAccumulation::Complete(body));
            }
            AnthropicEvent::Error { error } => {
                return Err(AnthropicStreamError::ProviderEvent {
                    kind: error.kind(),
                    message: error.message,
                });
            }
            AnthropicEvent::Ping | AnthropicEvent::Unknown => {}
        }
        Ok(AnthropicAccumulation::Continue)
    }

    fn require_started(&self, event: &'static str) -> Result<(), AnthropicStreamError> {
        if self.message_started {
            Ok(())
        } else {
            Err(AnthropicStreamError::EventBeforeMessageStart { event })
        }
    }

    fn complete_body(&self) -> Result<Value, AnthropicStreamError> {
        if self.blocks.values().any(|block| !block.stopped) {
            return Err(AnthropicStreamError::OpenContentBlocks);
        }
        let content = self
            .blocks
            .iter()
            .map(|(index, block)| block.body(*index))
            .collect::<Result<Vec<_>, _>>()?;
        let usage = self.usage.clone().unwrap_or_default();
        Ok(json!({
            "content": content,
            "stop_reason": self.stop_reason,
            "usage": {
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cache_read_input_tokens": usage.cache_read_input_tokens,
                "cache_creation_input_tokens": usage.cache_creation_input_tokens
            }
        }))
    }
}

impl AnthropicUsage {
    fn merge(&mut self, delta: AnthropicUsageDelta) {
        if let Some(value) = delta.input_tokens {
            self.input_tokens = value;
        }
        if let Some(value) = delta.output_tokens {
            self.output_tokens = value;
        }
        if let Some(value) = delta.cache_read_input_tokens {
            self.cache_read_input_tokens = value;
        }
        if let Some(value) = delta.cache_creation_input_tokens {
            self.cache_creation_input_tokens = value;
        }
    }
}
