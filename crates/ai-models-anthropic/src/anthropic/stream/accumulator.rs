//! Anthropic state validation and indexed event mapping.

use std::collections::BTreeMap;

use ai_interface::ModelEvent;

use super::{
    block::ContentBlockState,
    types::{
        AnthropicAccumulation, AnthropicCompletion, AnthropicEvent, AnthropicStreamError,
        AnthropicUsage, AnthropicUsageDelta, BlockEvent,
    },
};

#[derive(Debug, Default)]
pub(super) struct AnthropicStreamAccumulator {
    message_started: bool,
    message_stopped: bool,
    blocks: BTreeMap<u64, ContentBlockState>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
    assistant_blocks_emitted: usize,
    next_part_index: usize,
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
        let (index, events) = match event {
            AnthropicEvent::MessageStart { message } => {
                if self.message_started {
                    return Err(AnthropicStreamError::DuplicateMessageStart);
                }
                self.message_started = true;
                self.usage = Some(message.usage);
                return Ok(AnthropicAccumulation::Continue { events: Vec::new() });
            }
            AnthropicEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                self.require_started("content_block_start")?;
                let (block, event) = ContentBlockState::new(content_block);
                if self.blocks.insert(index, block).is_some() {
                    return Err(AnthropicStreamError::DuplicateContentBlock { index });
                }
                (index, event.into_iter().collect())
            }
            AnthropicEvent::ContentBlockDelta { index, delta } => {
                self.require_started("content_block_delta")?;
                let Some(block) = self.blocks.get_mut(&index) else {
                    return Err(AnthropicStreamError::UnknownContentBlock { index });
                };
                (index, block.push_delta(index, delta)?.into_iter().collect())
            }
            AnthropicEvent::ContentBlockStop { index } => {
                self.require_started("content_block_stop")?;
                let Some(block) = self.blocks.get_mut(&index) else {
                    return Err(AnthropicStreamError::UnknownContentBlock { index });
                };
                (index, block.stop(index)?)
            }
            AnthropicEvent::MessageDelta { delta, usage } => {
                self.require_started("message_delta")?;
                if let Some(stop_reason) = delta.stop_reason {
                    self.stop_reason = Some(stop_reason);
                }
                if let Some(current) = self.usage.as_mut() {
                    current.merge(usage);
                }
                return Ok(AnthropicAccumulation::Continue { events: Vec::new() });
            }
            AnthropicEvent::MessageStop => {
                self.require_started("message_stop")?;
                if self.blocks.values().any(|block| !block.stopped) {
                    return Err(AnthropicStreamError::OpenContentBlocks);
                }
                self.message_stopped = true;
                return Ok(AnthropicAccumulation::Complete(AnthropicCompletion {
                    blocks: self
                        .blocks
                        .values()
                        .filter_map(ContentBlockState::replay)
                        .collect(),
                    stop_reason: self.stop_reason.take(),
                    usage: self.usage.take().unwrap_or_default(),
                }));
            }
            AnthropicEvent::Error { error } => {
                return Err(AnthropicStreamError::ProviderEvent {
                    kind: error.kind(),
                    message: error.message,
                });
            }
            AnthropicEvent::Ping | AnthropicEvent::Unknown => {
                return Ok(AnthropicAccumulation::Continue { events: Vec::new() });
            }
        };
        Ok(AnthropicAccumulation::Continue {
            events: self.map_events(index, events)?,
        })
    }

    fn map_events(
        &mut self,
        index: u64,
        events: Vec<BlockEvent>,
    ) -> Result<Vec<ModelEvent>, AnthropicStreamError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }
        let Some(block) = self.blocks.get_mut(&index) else {
            return Err(AnthropicStreamError::UnknownContentBlock { index });
        };
        let part_index = match block.part_index {
            Some(index) => index,
            None => {
                let index = self.next_part_index;
                self.next_part_index = self.next_part_index.saturating_add(1);
                block.part_index = Some(index);
                index
            }
        };
        let mut result = Vec::new();
        for event in events {
            result.push(match event {
                BlockEvent::Text {
                    mut delta,
                    starts_block,
                } => {
                    if starts_block {
                        if self.assistant_blocks_emitted > 0 {
                            delta.insert(0, '\n');
                        }
                        self.assistant_blocks_emitted =
                            self.assistant_blocks_emitted.saturating_add(1);
                    }
                    ModelEvent::TextDelta { part_index, delta }
                }
                BlockEvent::Reasoning { delta } => ModelEvent::ReasoningDelta { part_index, delta },
                BlockEvent::ToolStart { id, name } => ModelEvent::ToolCallStart {
                    part_index,
                    id,
                    name,
                    operation_id: None,
                },
                BlockEvent::ToolArguments { delta } => {
                    ModelEvent::ToolCallArgumentsDelta { part_index, delta }
                }
                BlockEvent::ToolEnd => ModelEvent::ToolCallEnd { part_index },
            });
        }
        Ok(result)
    }

    fn require_started(&self, event: &'static str) -> Result<(), AnthropicStreamError> {
        if self.message_started {
            Ok(())
        } else {
            Err(AnthropicStreamError::EventBeforeMessageStart { event })
        }
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
