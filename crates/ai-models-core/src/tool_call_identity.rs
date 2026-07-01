//! Deterministic identity helpers for provider-synthesized tool calls.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest,
    ProviderConversationItem, StructuredOutputSchema, ToolCall, ToolDefinition,
};
use serde_json::Value;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Builds a deterministic scope for provider-synthesized tool-call ids.
pub fn synthetic_tool_call_scope(request: &ModelRequest) -> String {
    let mut hasher = StableHasher::new();
    hasher.write_str(&request.system_prompt);
    hasher.write_usize(request.messages.len());
    for message in &request.messages {
        hasher.write_message(message);
    }
    hasher.write_usize(request.tools.len());
    for tool in &request.tools {
        hasher.write_tool(tool);
    }
    hasher.write_optional_schema(request.response_schema.as_ref());
    hasher.finish_hex()
}

/// Builds a deterministic local id for a tool call with no provider id.
pub fn synthetic_tool_call_id(
    prefix: &str,
    request_scope: &str,
    call_index: usize,
    name: &str,
    arguments: &str,
) -> String {
    let mut hasher = StableHasher::new();
    hasher.write_str(prefix);
    hasher.write_str(request_scope);
    hasher.write_usize(call_index);
    hasher.write_str(name);
    hasher.write_str(arguments);
    format!("{prefix}{}", hasher.finish_hex())
}

/// Stable FNV-1a hasher used to avoid process-randomized ids.
struct StableHasher {
    value: u64,
}

impl StableHasher {
    fn new() -> Self {
        Self {
            value: FNV_OFFSET_BASIS,
        }
    }

    fn finish_hex(self) -> String {
        format!("{:016x}", self.value)
    }

    fn write_message(&mut self, message: &ConversationMessage) {
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
        }
    }

    fn write_provider_context(&mut self, item: &ProviderConversationItem) {
        match item {
            ProviderConversationItem::OpenAiMessage { phase } => {
                self.write_str("openai_message");
                self.write_optional_str(phase.as_deref());
            }
            ProviderConversationItem::OpenAiReasoning {
                id,
                summary,
                encrypted_content,
            } => {
                self.write_str("openai_reasoning");
                self.write_str(id);
                self.write_usize(summary.len());
                for item in summary {
                    self.write_str(&item.kind);
                    self.write_str(&item.text);
                }
                self.write_optional_str(encrypted_content.as_deref());
            }
            ProviderConversationItem::OpenAiFunctionCall {
                id,
                call_id,
                name,
                arguments,
            } => {
                self.write_str("openai_function_call");
                self.write_optional_str(id.as_deref());
                self.write_str(call_id);
                self.write_str(name);
                self.write_str(arguments);
            }
            ProviderConversationItem::XaiLegacyFunctionCall {
                tool_call_id,
                name,
                arguments,
            } => {
                self.write_str("xai_legacy_function_call");
                self.write_str(tool_call_id);
                self.write_str(name);
                self.write_str(arguments);
            }
        }
    }

    fn write_tool_call(&mut self, call: &ToolCall) {
        self.write_str(&call.id);
        self.write_str(&call.name);
        self.write_json(&call.input);
        self.write_optional_str(call.operation_id.as_deref());
    }

    fn write_tool(&mut self, tool: &ToolDefinition) {
        self.write_str(&tool.name);
        self.write_str(&tool.description);
        self.write_json(&tool.input_schema);
    }

    fn write_optional_schema(&mut self, schema: Option<&StructuredOutputSchema>) {
        match schema {
            Some(schema) => {
                self.write_bool(true);
                self.write_str(&schema.name);
                self.write_json(&schema.schema);
            }
            None => self.write_bool(false),
        }
    }

    fn write_json(&mut self, value: &Value) {
        self.write_str(&value.to_string());
    }

    fn write_optional_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.write_bool(true);
                self.write_str(value);
            }
            None => self.write_bool(false),
        }
    }

    fn write_str(&mut self, value: &str) {
        self.write_usize(value.len());
        self.write_bytes(value.as_bytes());
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_bool(&mut self, value: bool) {
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
