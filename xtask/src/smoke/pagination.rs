//! Universal tool-output pagination smoke fixtures.

use std::sync::atomic::{AtomicUsize, Ordering};

use ai_interface::{
    ConversationRole, FinishReason, Model, ModelRequest, ModelResponse, ModelResult, ModelUsage,
    Tool, ToolCall, ToolDefinition, ToolOutputEnvelope,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use thiserror::Error as ThisError;

pub(super) struct SmokePaginationModel {
    calls: AtomicUsize,
}

impl SmokePaginationModel {
    pub(super) fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Model for SmokePaginationModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        if !request
            .tools
            .iter()
            .any(|definition| definition.name == "tool_output_read")
        {
            return Err(ai_interface::ModelError::internal(
                SmokeModelError::MissingReaderDefinition,
            ));
        }
        let step = self.calls.fetch_add(1, Ordering::SeqCst);
        match step {
            0 => Ok(tool_call_response(tool_call(
                "smoke-big-call",
                "smoke_big_output",
                json!({}),
            ))),
            1 | 2 => self.next_read_response(request, step),
            3 => self.stop_after_final_window(request),
            _ => Err(ai_interface::ModelError::internal(
                SmokeModelError::UnexpectedModelStep { step },
            )),
        }
    }
}

impl SmokePaginationModel {
    fn next_read_response(
        &self,
        request: &ModelRequest,
        step: usize,
    ) -> ModelResult<ModelResponse> {
        let window = last_tool_window(request)?;
        let Some(output_id) = window.output_id().cloned() else {
            return Err(ai_interface::ModelError::internal(
                SmokeModelError::MissingWindowOutputId,
            ));
        };
        let Some(next_offset) = window.next_offset() else {
            return Err(ai_interface::ModelError::internal(
                SmokeModelError::MissingNextOffset { step },
            ));
        };
        Ok(tool_call_response(tool_call(
            match step {
                1 => "smoke-read-1",
                _ => "smoke-read-2",
            },
            "tool_output_read",
            json!({
                "output_id": output_id,
                "offset": next_offset
            }),
        )))
    }

    fn stop_after_final_window(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let window = last_tool_window(request)?;
        if window.truncated() {
            return Err(ai_interface::ModelError::internal(
                SmokeModelError::UnexpectedTruncatedFinalWindow,
            ));
        }
        Ok(ModelResponse {
            provider: "smoke".to_owned(),
            model_id: "smoke-model".to_owned(),
            catalog_model_id: None,
            thinking_level: None,
            assistant_message: "done".to_owned(),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            structured_output: None,
            provider_context: Vec::new(),
            usage: ModelUsage::default(),
        })
    }
}

pub(super) struct SmokeTool;

#[async_trait]
impl Tool for SmokeTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "smoke_big_output".to_owned(),
            description: "Return a large deterministic payload.".to_owned(),
            input_schema: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {}
            }),
            activity_verb: Some("Checking".to_owned()),
        }]
    }

    async fn call(&self, tool_name: &str, input: Value) -> ai_interface::ToolResult<Value> {
        let _ = input;
        if tool_name != "smoke_big_output" {
            return Err(ai_interface::ToolError::UnknownTool {
                name: tool_name.to_owned(),
            });
        }
        Ok(Value::String("a".repeat(45_000)))
    }

    fn group_for_tool(&self, tool_name: &str) -> Option<&'static str> {
        let _ = tool_name;
        Some("smoke")
    }
}

fn last_tool_window(request: &ModelRequest) -> ModelResult<ai_interface::ToolOutputWindowEnvelope> {
    let Some(message) = request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ConversationRole::Tool)
    else {
        return Err(ai_interface::ModelError::internal(
            SmokeModelError::MissingToolWindow,
        ));
    };
    let envelope = match serde_json::from_str::<ToolOutputEnvelope>(&message.content) {
        Ok(envelope) => envelope,
        Err(source) => {
            return Err(ai_interface::ModelError::internal(
                SmokeModelError::InvalidEnvelope { source },
            ));
        }
    };
    match envelope {
        ToolOutputEnvelope::Window(window) => Ok(window),
        ToolOutputEnvelope::Inline(_) => Err(ai_interface::ModelError::internal(
            SmokeModelError::UnexpectedInlineEnvelope,
        )),
    }
}

fn tool_call(id: &str, name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input,
        operation_id: None,
    }
}

fn tool_call_response(call: ToolCall) -> ModelResponse {
    ModelResponse {
        provider: "smoke".to_owned(),
        model_id: "smoke-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "calling tool".to_owned(),
        tool_calls: vec![call],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}

#[derive(Debug, ThisError)]
pub(super) enum SmokeModelError {
    #[error("[xtask/smoke] tool_output_read definition was missing")]
    MissingReaderDefinition,
    #[error("[xtask/smoke] expected a tool output window in conversation")]
    MissingToolWindow,
    #[error("[xtask/smoke] failed to decode tool output envelope: {source}")]
    InvalidEnvelope { source: serde_json::Error },
    #[error("[xtask/smoke] expected a window envelope")]
    UnexpectedInlineEnvelope,
    #[error("[xtask/smoke] window did not carry an output id")]
    MissingWindowOutputId,
    #[error("[xtask/smoke] step {step} did not provide a next offset")]
    MissingNextOffset { step: usize },
    #[error("[xtask/smoke] final read window was still truncated")]
    UnexpectedTruncatedFinalWindow,
    #[error("[xtask/smoke] model was called for unexpected step {step}")]
    UnexpectedModelStep { step: usize },
    #[error("[xtask/smoke] runtime did not complete the expected pagination flow")]
    UnexpectedOutcome,
}
