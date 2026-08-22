//! Typed state for one indexed Anthropic content block.

use serde_json::{Value, json};

use super::types::{AnthropicContentBlockDelta, AnthropicContentBlockStart, AnthropicStreamError};

#[derive(Debug)]
pub(super) struct ContentBlockState {
    content: ContentBlockContent,
    pub(super) stopped: bool,
}

impl ContentBlockState {
    pub(super) fn new(block: AnthropicContentBlockStart) -> Self {
        let content = match block {
            AnthropicContentBlockStart::Text { text } => ContentBlockContent::Text(text),
            AnthropicContentBlockStart::ToolUse { id, name, input } => {
                ContentBlockContent::ToolUse {
                    id,
                    name,
                    input,
                    partial_json: String::new(),
                }
            }
            AnthropicContentBlockStart::Thinking {
                thinking,
                signature,
            } => ContentBlockContent::Thinking {
                thinking,
                signature,
            },
            AnthropicContentBlockStart::Ignored => ContentBlockContent::Ignored,
        };
        Self {
            content,
            stopped: false,
        }
    }

    pub(super) fn push_delta(
        &mut self,
        index: u64,
        delta: AnthropicContentBlockDelta,
    ) -> Result<(), AnthropicStreamError> {
        if self.stopped {
            return Err(AnthropicStreamError::ContentBlockAfterStop { index });
        }
        match (&mut self.content, delta) {
            (ContentBlockContent::Text(text), AnthropicContentBlockDelta::Text { text: delta }) => {
                text.push_str(&delta);
            }
            (
                ContentBlockContent::ToolUse { partial_json, .. },
                AnthropicContentBlockDelta::InputJson {
                    partial_json: delta,
                },
            ) => partial_json.push_str(&delta),
            (
                ContentBlockContent::Thinking { thinking, .. },
                AnthropicContentBlockDelta::Thinking { thinking: delta },
            ) => thinking.push_str(&delta),
            (
                ContentBlockContent::Thinking { signature, .. },
                AnthropicContentBlockDelta::Signature { signature: delta },
            ) => signature.push_str(&delta),
            (ContentBlockContent::Ignored, _) | (_, AnthropicContentBlockDelta::Ignored) => {}
            (content, delta) => {
                return Err(AnthropicStreamError::MismatchedBlockDelta {
                    index,
                    block_kind: content.kind(),
                    delta_kind: delta.kind(),
                });
            }
        }
        Ok(())
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
            ContentBlockContent::Text(text) => json!({"type": "text", "text": text}),
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
    Text(String),
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
            Self::Text(_) => "text",
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
