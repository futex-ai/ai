//! Stable field hashing for normalized requests.

use ai_interface::{ConversationPart, ConversationRole, StructuredOutputSchema, ToolDefinition};
use serde_json::Value;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Stable FNV-1a hasher used to avoid process-randomized ids.
pub(super) struct StableHasher {
    value: u64,
}

impl StableHasher {
    pub(super) fn new() -> Self {
        Self {
            value: FNV_OFFSET_BASIS,
        }
    }

    pub(super) fn finish_hex(self) -> String {
        format!("{:016x}", self.value)
    }

    pub(super) fn write_role(&mut self, role: ConversationRole) {
        match role {
            ConversationRole::User => self.write_str("user"),
            ConversationRole::Assistant => self.write_str("assistant"),
            ConversationRole::Tool => self.write_str("tool"),
        }
    }

    pub(super) fn write_part(&mut self, part: &ConversationPart) {
        match part {
            ConversationPart::Text { text, .. } => {
                self.write_str("text");
                self.write_str(text);
            }
            ConversationPart::Image {
                mime_type,
                data_base64,
                ..
            } => {
                self.write_str("image");
                self.write_str(mime_type);
                self.write_str(data_base64);
            }
            ConversationPart::Video {
                mime_type,
                data_base64,
                ..
            } => {
                self.write_str("video");
                self.write_str(mime_type);
                self.write_str(data_base64);
            }
            ConversationPart::Reasoning { text, .. } => {
                self.write_str("reasoning");
                self.write_str(text);
            }
            ConversationPart::ToolCall {
                call_id,
                tool_name,
                arguments,
                operation_id,
                ..
            } => {
                self.write_str("tool_call");
                self.write_str(call_id);
                self.write_str(tool_name);
                self.write_json(arguments);
                self.write_optional_str(operation_id.as_deref());
            }
            ConversationPart::ToolResult {
                call_id,
                tool_name,
                content,
                ..
            } => {
                self.write_str("tool_result");
                self.write_str(call_id);
                self.write_str(tool_name);
                self.write_str(content);
            }
        }
    }

    pub(super) fn write_tool(&mut self, tool: &ToolDefinition) {
        self.write_str(&tool.name);
        self.write_str(&tool.description);
        self.write_json(&tool.input_schema);
    }

    pub(super) fn write_optional_schema(&mut self, schema: Option<&StructuredOutputSchema>) {
        match schema {
            Some(schema) => {
                self.write_bool(true);
                self.write_str(&schema.name);
                self.write_json(&schema.schema);
            }
            None => self.write_bool(false),
        }
    }

    pub(super) fn write_json(&mut self, value: &Value) {
        self.write_str(&value.to_string());
    }

    pub(super) fn write_optional_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.write_bool(true);
                self.write_str(value);
            }
            None => self.write_bool(false),
        }
    }

    pub(super) fn write_str(&mut self, value: &str) {
        self.write_usize(value.len());
        self.write_bytes(value.as_bytes());
    }

    pub(super) fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    pub(super) fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub(super) fn write_bool(&mut self, value: bool) {
        self.write_byte(u8::from(value));
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_byte(*byte);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        self.value ^= u64::from(byte);
        self.value = self.value.wrapping_mul(FNV_PRIME);
    }
}
