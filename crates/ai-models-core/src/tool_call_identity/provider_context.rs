//! Stable hashing for provider-owned replay context.

use ai_interface::{AnthropicAssistantBlock, GoogleAssistantPart, ProviderConversationItem};

use super::hasher::StableHasher;

impl StableHasher {
    pub(super) fn write_provider_context(&mut self, item: &ProviderConversationItem) {
        match item {
            ProviderConversationItem::AnthropicAssistantMessage { blocks } => {
                self.write_str("anthropic_assistant_message");
                self.write_usize(blocks.len());
                for block in blocks {
                    self.write_anthropic_block(block);
                }
            }
            ProviderConversationItem::GoogleAssistantMessage { parts } => {
                self.write_str("google_assistant_message");
                self.write_usize(parts.len());
                for part in parts {
                    self.write_google_part(part);
                }
            }
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
            ProviderConversationItem::MiniMaxAssistant {
                reasoning_content,
                reasoning_details,
            } => {
                self.write_str("minimax_assistant");
                self.write_optional_str(reasoning_content.as_deref());
                self.write_usize(reasoning_details.len());
                for detail in reasoning_details {
                    self.write_optional_str(detail.kind.as_deref());
                    self.write_optional_str(detail.id.as_deref());
                    self.write_optional_str(detail.format.as_deref());
                    match detail.index {
                        Some(index) => {
                            self.write_bool(true);
                            self.write_u64(u64::from(index));
                        }
                        None => self.write_bool(false),
                    }
                    self.write_optional_str(detail.text.as_deref());
                }
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
            ProviderConversationItem::DeepSeekAssistantMessage {
                content,
                reasoning_content,
                tool_calls,
            } => {
                self.write_str("deep_seek_assistant_message");
                self.write_str(content);
                self.write_optional_str(reasoning_content.as_deref());
                self.write_usize(tool_calls.len());
                for call in tool_calls {
                    self.write_str(&call.id);
                    self.write_str(&call.name);
                    self.write_str(&call.arguments);
                }
            }
            ProviderConversationItem::KimiAssistantMessage {
                content,
                reasoning_content,
                tool_calls,
            } => {
                self.write_str("kimi_assistant_message");
                self.write_optional_str(content.as_deref());
                self.write_optional_str(reasoning_content.as_deref());
                self.write_usize(tool_calls.len());
                for call in tool_calls {
                    self.write_str(&call.id);
                    self.write_str(&call.name);
                    self.write_str(&call.arguments);
                }
            }
            ProviderConversationItem::QwenAssistantMessage {
                content,
                reasoning_content,
                tool_calls,
            } => {
                self.write_str("qwen_assistant_message");
                self.write_optional_str(content.as_deref());
                self.write_optional_str(reasoning_content.as_deref());
                self.write_usize(tool_calls.len());
                for call in tool_calls {
                    self.write_str(&call.id);
                    self.write_str(&call.name);
                    self.write_str(&call.arguments);
                }
            }
        }
    }
}

impl StableHasher {
    fn write_anthropic_block(&mut self, block: &AnthropicAssistantBlock) {
        match block {
            AnthropicAssistantBlock::Thinking {
                thinking,
                signature,
            } => {
                self.write_str("thinking");
                self.write_str(thinking);
                self.write_str(signature);
            }
            AnthropicAssistantBlock::RedactedThinking { data } => {
                self.write_str("redacted_thinking");
                self.write_str(data);
            }
            AnthropicAssistantBlock::Text { text } => {
                self.write_str("text");
                self.write_str(text);
            }
            AnthropicAssistantBlock::ToolUse { id, name, input } => {
                self.write_str("tool_use");
                self.write_str(id);
                self.write_str(name);
                self.write_json(input);
            }
        }
    }

    fn write_google_part(&mut self, part: &GoogleAssistantPart) {
        self.write_optional_str(part.text.as_deref());
        match part.thought {
            Some(thought) => {
                self.write_bool(true);
                self.write_bool(thought);
            }
            None => self.write_bool(false),
        }
        match &part.function_call {
            Some(call) => {
                self.write_bool(true);
                self.write_optional_str(call.id.as_deref());
                self.write_str(&call.name);
                match &call.args {
                    Some(args) => {
                        self.write_bool(true);
                        self.write_json(args);
                    }
                    None => self.write_bool(false),
                }
            }
            None => self.write_bool(false),
        }
        self.write_optional_str(part.thought_signature.as_deref());
    }
}
