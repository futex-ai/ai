//! Stable field hashing for normalized requests.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, StructuredOutputSchema,
    ToolCall, ToolDefinition,
};
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

    pub(super) fn write_message(&mut self, message: &ConversationMessage) {
        self.write_role(message.role);
        self.write_str(&message.content);
        self.write_usize(message.content_parts.len());
        for part in &message.content_parts {
            self.write_content_part(part);
        }
        self.write_optional_str(message.name.as_deref());
        self.write_optional_str(message.tool_call_id.as_deref());
        self.write_usize(message.tool_calls.len());
        for call in &message.tool_calls {
            self.write_tool_call(call);
        }
        self.write_usize(message.provider_context.len());
        for item in &message.provider_context {
            self.write_provider_context(item);
        }
    }

    fn write_role(&mut self, role: ConversationRole) {
        match role {
            ConversationRole::User => self.write_str("user"),
            ConversationRole::Assistant => self.write_str("assistant"),
            ConversationRole::Tool => self.write_str("tool"),
        }
    }

    fn write_content_part(&mut self, part: &ConversationContentPart) {
        match part {
            ConversationContentPart::Text { text } => {
                self.write_str("text");
                self.write_str(text);
            }
            ConversationContentPart::Image {
                mime_type,
                data_base64,
            } => {
                self.write_str("image");
                self.write_str(mime_type);
                self.write_str(data_base64);
            }
            ConversationContentPart::Video {
                mime_type,
                data_base64,
            } => {
                self.write_str("video");
                self.write_str(mime_type);
                self.write_str(data_base64);
            }
        }
    }

    fn write_tool_call(&mut self, call: &ToolCall) {
        self.write_str(&call.id);
        self.write_str(&call.name);
        self.write_json(&call.input);
        self.write_optional_str(call.operation_id.as_deref());
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
