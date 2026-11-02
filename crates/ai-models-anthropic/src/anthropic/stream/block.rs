//! Typed state for one indexed Anthropic content block.

use ai_interface::AnthropicAssistantBlock;
use serde_json::Value;

use super::types::{
    AnthropicContentBlockDelta, AnthropicContentBlockStart, AnthropicStreamError, BlockEvent,
};

#[derive(Debug)]
pub(super) struct ContentBlockState {
    content: ContentBlockContent,
    pub(super) stopped: bool,
    pub(super) part_index: Option<usize>,
}

impl ContentBlockState {
    pub(super) fn new(block: AnthropicContentBlockStart) -> (Self, Option<BlockEvent>) {
        let (content, event) = match block {
            AnthropicContentBlockStart::Text { text } => {
                let emitted = !text.trim().is_empty();
                let event = emitted.then(|| BlockEvent::Text {
                    delta: text.clone(),
                    starts_block: true,
                });
                (ContentBlockContent::Text { text, emitted }, event)
            }
            AnthropicContentBlockStart::ToolUse { id, name, input } => {
                let event = Some(BlockEvent::ToolStart {
                    id: id.clone(),
                    name: name.clone(),
                });
                (
                    ContentBlockContent::ToolUse {
                        id,
                        name,
                        input,
                        partial_json: String::new(),
                    },
                    event,
                )
            }
            AnthropicContentBlockStart::Thinking {
                thinking,
                signature,
            } => {
                let event = (!thinking.is_empty()).then(|| BlockEvent::Reasoning {
                    delta: thinking.clone(),
                });
                (
                    ContentBlockContent::Thinking {
                        thinking,
                        signature,
                    },
                    event,
                )
            }
            AnthropicContentBlockStart::RedactedThinking { data } => {
                (ContentBlockContent::RedactedThinking { data }, None)
            }
            AnthropicContentBlockStart::Ignored => (ContentBlockContent::Ignored, None),
        };
        (
            Self {
                content,
                stopped: false,
                part_index: None,
            },
            event,
        )
    }

    pub(super) fn push_delta(
        &mut self,
        index: u64,
        delta: AnthropicContentBlockDelta,
    ) -> Result<Option<BlockEvent>, AnthropicStreamError> {
        if self.stopped {
            return Err(AnthropicStreamError::ContentBlockAfterStop { index });
        }
        let event = match (&mut self.content, delta) {
            (
                ContentBlockContent::Text { text, emitted },
                AnthropicContentBlockDelta::Text { text: delta },
            ) => {
                text.push_str(&delta);
                if delta.is_empty() {
                    None
                } else if *emitted {
                    Some(BlockEvent::Text {
                        delta,
                        starts_block: false,
                    })
                } else if text.trim().is_empty() {
                    None
                } else {
                    *emitted = true;
                    Some(BlockEvent::Text {
                        delta: text.clone(),
                        starts_block: true,
                    })
                }
            }
            (
                ContentBlockContent::ToolUse { partial_json, .. },
                AnthropicContentBlockDelta::InputJson {
                    partial_json: delta,
                },
            ) => {
                partial_json.push_str(&delta);
                (!delta.is_empty()).then_some(BlockEvent::ToolArguments { delta })
            }
            (
                ContentBlockContent::Thinking { thinking, .. },
                AnthropicContentBlockDelta::Thinking { thinking: delta },
            ) => {
                thinking.push_str(&delta);
                (!delta.is_empty()).then_some(BlockEvent::Reasoning { delta })
            }
            (
                ContentBlockContent::Thinking { signature, .. },
                AnthropicContentBlockDelta::Signature { signature: delta },
            ) => {
                signature.push_str(&delta);
                None
            }
            (ContentBlockContent::Ignored, _) | (_, AnthropicContentBlockDelta::Ignored) => None,
            (content, delta) => {
                return Err(AnthropicStreamError::MismatchedBlockDelta {
                    index,
                    block_kind: content.kind(),
                    delta_kind: delta.kind(),
                });
            }
        };
        Ok(event)
    }

    pub(super) fn stop(&mut self, index: u64) -> Result<Vec<BlockEvent>, AnthropicStreamError> {
        if self.stopped {
            return Err(AnthropicStreamError::DuplicateContentBlockStop { index });
        }
        let mut events = Vec::new();
        if let ContentBlockContent::ToolUse {
            input,
            partial_json,
            ..
        } = &mut self.content
        {
            if partial_json.is_empty() {
                events.push(BlockEvent::ToolArguments {
                    delta: input.to_string(),
                });
            } else {
                *input = match serde_json::from_str(partial_json) {
                    Ok(input) => input,
                    Err(source) => {
                        return Err(AnthropicStreamError::InvalidToolInput { index, source });
                    }
                };
            }
            events.push(BlockEvent::ToolEnd);
        }
        self.stopped = true;
        Ok(events)
    }

    pub(super) fn replay(&self) -> Option<AnthropicAssistantBlock> {
        match &self.content {
            ContentBlockContent::Text { text, .. } => {
                Some(AnthropicAssistantBlock::Text { text: text.clone() })
            }
            ContentBlockContent::ToolUse {
                id, name, input, ..
            } => Some(AnthropicAssistantBlock::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            ContentBlockContent::Thinking {
                thinking,
                signature,
            } => Some(AnthropicAssistantBlock::Thinking {
                thinking: thinking.clone(),
                signature: signature.clone(),
            }),
            ContentBlockContent::RedactedThinking { data } => {
                Some(AnthropicAssistantBlock::RedactedThinking { data: data.clone() })
            }
            ContentBlockContent::Ignored => None,
        }
    }
}

#[derive(Debug)]
enum ContentBlockContent {
    Text {
        text: String,
        emitted: bool,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
        partial_json: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
    Ignored,
}

impl ContentBlockContent {
    fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::ToolUse { .. } => "tool_use",
            Self::Thinking { .. } => "thinking",
            Self::RedactedThinking { .. } => "redacted_thinking",
            Self::Ignored => "ignored",
        }
    }
}

impl AnthropicContentBlockDelta {
    fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text_delta",
            Self::InputJson { .. } => "input_json_delta",
            Self::Thinking { .. } => "thinking_delta",
            Self::Signature { .. } => "signature_delta",
            Self::Ignored => "ignored",
        }
    }
}
