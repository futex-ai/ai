//! Typed state for one indexed Anthropic content block.

use serde_json::{Value, json};

use super::types::{
    AnthropicContentBlockDelta, AnthropicContentBlockStart, AnthropicStreamDelta,
    AnthropicStreamError,
};

#[derive(Debug)]
pub(super) struct ContentBlockState {
    content: ContentBlockContent,
    pub(super) stopped: bool,
}

impl ContentBlockState {
    pub(super) fn new(block: AnthropicContentBlockStart) -> (Self, Option<AnthropicStreamDelta>) {
        let (content, delta) = match block {
            AnthropicContentBlockStart::Text { text } => {
                let emitted = !text.trim().is_empty();
                let delta = emitted.then(|| AnthropicStreamDelta::AssistantText {
                    delta: text.clone(),
                    starts_block: true,
                });
                (ContentBlockContent::Text { text, emitted }, delta)
            }
            AnthropicContentBlockStart::ToolUse { id, name, input } => (
                ContentBlockContent::ToolUse {
                    id,
                    name,
                    input,
                    partial_json: String::new(),
                },
                None,
            ),
            AnthropicContentBlockStart::Thinking {
                thinking,
                signature,
            } => {
                let delta = (!thinking.is_empty()).then(|| AnthropicStreamDelta::ReasoningText {
                    delta: thinking.clone(),
                });
                (
                    ContentBlockContent::Thinking {
                        thinking,
                        signature,
                    },
                    delta,
                )
            }
            AnthropicContentBlockStart::Ignored => (ContentBlockContent::Ignored, None),
        };
        (
            Self {
                content,
                stopped: false,
            },
            delta,
        )
    }

    pub(super) fn push_delta(
        &mut self,
        index: u64,
        delta: AnthropicContentBlockDelta,
    ) -> Result<Option<AnthropicStreamDelta>, AnthropicStreamError> {
        if self.stopped {
            return Err(AnthropicStreamError::ContentBlockAfterStop { index });
        }
        let stream_delta = match (&mut self.content, delta) {
            (
                ContentBlockContent::Text { text, emitted },
                AnthropicContentBlockDelta::Text { text: delta },
            ) => {
                text.push_str(&delta);
                if delta.is_empty() {
                    None
                } else if *emitted {
                    Some(AnthropicStreamDelta::AssistantText {
                        delta,
                        starts_block: false,
                    })
                } else if text.trim().is_empty() {
                    None
                } else {
                    *emitted = true;
                    Some(AnthropicStreamDelta::AssistantText {
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
                None
            }
            (
                ContentBlockContent::Thinking { thinking, .. },
                AnthropicContentBlockDelta::Thinking { thinking: delta },
            ) => {
                thinking.push_str(&delta);
                (!delta.is_empty()).then_some(AnthropicStreamDelta::ReasoningText { delta })
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
        Ok(stream_delta)
    }

    pub(super) fn stop(&mut self, index: u64) -> Result<(), AnthropicStreamError> {
        if self.stopped {
            return Err(AnthropicStreamError::DuplicateContentBlockStop { index });
        }
        if let ContentBlockContent::ToolUse {
            input,
            partial_json,
            ..
        } = &mut self.content
            && !partial_json.is_empty()
        {
            *input = match serde_json::from_str(partial_json) {
                Ok(input) => input,
                Err(source) => {
                    return Err(AnthropicStreamError::InvalidToolInput { index, source });
                }
            };
        }
        self.stopped = true;
        Ok(())
    }

    pub(super) fn body(&self, index: u64) -> Result<Value, AnthropicStreamError> {
        if !self.stopped {
            return Err(AnthropicStreamError::OpenContentBlocks);
        }
        let body = match &self.content {
            ContentBlockContent::Text { text, .. } => json!({"type": "text", "text": text}),
            ContentBlockContent::ToolUse {
                id, name, input, ..
            } => json!({"type": "tool_use", "id": id, "name": name, "input": input}),
            ContentBlockContent::Thinking {
                thinking,
                signature,
            } => json!({
                "type": "thinking",
                "thinking": thinking,
                "signature": signature
            }),
            ContentBlockContent::Ignored => json!({"type": "ignored", "index": index}),
        };
        Ok(body)
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
    Ignored,
}

impl ContentBlockContent {
    fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::ToolUse { .. } => "tool_use",
            Self::Thinking { .. } => "thinking",
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
