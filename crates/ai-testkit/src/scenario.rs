//! Shared requests for model and agent provider conformance scenarios.

use ai_interface::{
    ConversationMessage, ConversationPart, ModelRequest, ModelResponse, ModelToolChoice,
    StructuredOutputSchema, ToolDefinition,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{Error, Result};

/// A stable, portable provider behavior probe.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    /// One plain text response.
    TextTurn,
    /// A thinking-enabled tool call followed by a final answer.
    ToolLoop,
    /// A thinking-enabled tool call continued by the agent loop.
    AgentToolLoop,
    /// A response constrained by a JSON schema.
    SchemaCall,
    /// A user message containing a small PNG.
    ImageInput,
    /// A documented HTTP 429 response.
    RateLimit,
    /// Transport interruption after generation starts.
    StreamCut,
}

impl Scenario {
    /// Model-level scenarios in stable execution order.
    pub const ALL: [Self; 6] = [
        Self::TextTurn,
        Self::ToolLoop,
        Self::SchemaCall,
        Self::ImageInput,
        Self::RateLimit,
        Self::StreamCut,
    ];

    /// Agent scenarios, with separate recordings for automatic tool continuations.
    pub const AGENT: [Self; 2] = [Self::TextTurn, Self::AgentToolLoop];

    /// Scenarios that can be recorded with live success responses.
    pub const SUCCESS: [Self; 4] = [
        Self::TextTurn,
        Self::ToolLoop,
        Self::SchemaCall,
        Self::ImageInput,
    ];

    /// Stable filename stem.
    pub fn name(self) -> &'static str {
        match self {
            Self::TextTurn => "text_turn",
            Self::ToolLoop => "tool_loop",
            Self::AgentToolLoop => "agent_tool_loop",
            Self::SchemaCall => "schema_call",
            Self::ImageInput => "image_input",
            Self::RateLimit => "rate_limit",
            Self::StreamCut => "stream_cut",
        }
    }

    /// Builds the first request using today's model contracts.
    pub fn request(self) -> ModelRequest {
        let mut request = ModelRequest {
            system_prompt: "Follow the user's instructions. Keep answers short.".to_owned(),
            messages: vec![ConversationMessage::user("Reply with exactly Done.")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        match self {
            Self::ToolLoop | Self::AgentToolLoop => {
                request.messages = vec![ConversationMessage::user(
                    "Call lookup with id 7. After its result, reply with exactly Done.",
                )];
                request.tools = vec![ToolDefinition {
                    name: "lookup".to_owned(),
                    description: "Look up one number.".to_owned(),
                    input_schema: json!({"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"], "additionalProperties": false}),
                    activity_verb: None,
                }];
                request.controls.generation.tool_choice = Some(ModelToolChoice::Auto);
            }
            Self::SchemaCall => {
                request.messages = vec![ConversationMessage::user(
                    "Return a JSON object with ok set to true.",
                )];
                request.response_schema = Some(StructuredOutputSchema {
                    name: "result".to_owned(),
                    schema: json!({"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false}),
                });
            }
            Self::ImageInput => {
                let prompt = "This image is a small test PNG. Reply with exactly Done.";
                request.messages = vec![ConversationMessage::user_with_parts(vec![
                    ConversationPart::text(prompt),
                    ConversationPart::Image {
                        mime_type: "image/png".to_owned(),
                        provider_state: None,
                        data_base64: "iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAAAKElEQVR4nO3NsQ0AAAzCMP5/un0CNkuZ41wybXsHAAAAAAAAAAAAxR4yw/wuPL6QkAAAAABJRU5ErkJggg==".to_owned(),
                    },
                ])];
            }
            _ => {}
        }
        request
    }

    /// Preserves the first assistant message and its ordered parts and provider state on step two.
    pub fn follow_up(self, response: &ModelResponse) -> Result<ModelRequest> {
        if response.tool_calls().is_empty() {
            return Err(Error::MissingToolCall);
        }
        let mut request = self.request();
        request.messages.push(response.to_assistant_message());
        for call in response.tool_calls() {
            request.messages.push(ConversationMessage::tool(
                "{\"value\":7}",
                &call.name,
                &call.id,
            ));
        }
        request.messages.push(ConversationMessage::user(
            "Use the tool result. Reply with exactly Done.",
        ));
        request.controls.generation.tool_choice = Some(ModelToolChoice::None);
        Ok(request)
    }
}

#[cfg(test)]
#[path = "_tests_/scenario_tests.rs"]
mod scenario_tests;
