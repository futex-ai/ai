//! Tool dispatch helper for active turns.

use std::time::Instant;

use ai_interface::{
    ConversationMessage, DynTool, ToolActivityLogEntry, ToolActivityPhase, ToolCall,
    ToolCallLogEntry, ToolCallLogResult, ToolError, ToolInvocation, ToolOutputReadRequest,
};

use crate::intrinsic::is_intrinsic_tool;
use crate::tool_output::{
    TOOL_OUTPUT_READ, envelope_json, envelope_value, manage_successful_tool_output,
    readable_envelope,
};
use crate::{Error, Result, ToolCallingRuntime, ToolOutputStoreReadRequest};

use super::{
    failures::{tool_error_log_result, tool_error_message},
    types::ToolExecutionRecord,
};

pub(super) async fn dispatch_tool_call(
    runtime: &ToolCallingRuntime,
    call: ToolCall,
) -> Result<ToolExecutionRecord> {
    if is_intrinsic_tool(&call.name) {
        return dispatch_intrinsic_output_read(runtime, call).await;
    }

    let Some(tool) = runtime.tool_for_name(&call.name) else {
        let error = ToolError::UnknownTool {
            name: call.name.clone(),
        };
        runtime.append_message(tool_error_message(&call, &error));
        runtime.logger.log_tool_call(&ToolCallLogEntry {
            call,
            tool_group: None,
            result: tool_error_log_result(&error),
            latency_ms: 0,
        })?;
        return Err(Error::Tool(error));
    };

    dispatch_registered_tool(runtime, call, tool).await
}

async fn dispatch_registered_tool(
    runtime: &ToolCallingRuntime,
    call: ToolCall,
    tool: DynTool,
) -> Result<ToolExecutionRecord> {
    let tool_name = call.name.clone();
    let activity_verb = runtime.activity_verb_for_name(&tool_name);
    let tool_group = runtime.tool_group_for_name(&tool_name);
    runtime.logger.log_tool_activity(&ToolActivityLogEntry {
        tool_name: tool_name.clone(),
        activity_verb: activity_verb.clone(),
        phase: ToolActivityPhase::Started,
    })?;
    let operation_id = call.operation_id.clone().unwrap_or_else(|| call.id.clone());
    let started_at = Instant::now();
    match tool
        .call_with_invocation(ToolInvocation {
            tool_name: call.name.clone(),
            input: call.input.clone(),
            operation_id: operation_id.clone(),
        })
        .await
    {
        Ok(raw_output) => {
            let managed = manage_successful_tool_output(runtime, &call.name, raw_output).await?;
            let record = ToolExecutionRecord {
                call_id: call.id.clone(),
                operation_id,
                name: call.name.clone(),
                output_id: managed.output_id,
                raw_output: managed.raw_output,
                model_visible_output: managed.model_visible_output,
            };
            let content = envelope_json(&record.model_visible_output, &call.name)?;
            runtime.append_message(ConversationMessage::tool(
                content,
                call.name.clone(),
                call.id.clone(),
            ));
            runtime.logger.log_tool_call(&ToolCallLogEntry {
                call,
                tool_group,
                result: ToolCallLogResult::Success {
                    output: record.model_visible_output.clone(),
                },
                latency_ms: started_at.elapsed().as_millis(),
            })?;
            runtime.logger.log_tool_activity(&ToolActivityLogEntry {
                tool_name,
                activity_verb,
                phase: ToolActivityPhase::Completed,
            })?;
            Ok(record)
        }
        Err(error) => {
            runtime.append_message(tool_error_message(&call, &error));
            runtime.logger.log_tool_call(&ToolCallLogEntry {
                call,
                tool_group,
                result: tool_error_log_result(&error),
                latency_ms: started_at.elapsed().as_millis(),
            })?;
            runtime.logger.log_tool_activity(&ToolActivityLogEntry {
                tool_name,
                activity_verb,
                phase: ToolActivityPhase::Completed,
            })?;
            Err(Error::Tool(error))
        }
    }
}

async fn dispatch_intrinsic_output_read(
    runtime: &ToolCallingRuntime,
    call: ToolCall,
) -> Result<ToolExecutionRecord> {
    let tool_name = call.name.clone();
    let activity_verb = runtime.activity_verb_for_name(&tool_name);
    runtime.logger.log_tool_activity(&ToolActivityLogEntry {
        tool_name: tool_name.clone(),
        activity_verb: activity_verb.clone(),
        phase: ToolActivityPhase::Started,
    })?;
    let operation_id = call.operation_id.clone().unwrap_or_else(|| call.id.clone());
    let started_at = Instant::now();
    match read_intrinsic_output(runtime, &call, operation_id.clone()).await {
        Ok(record) => {
            let content = envelope_json(&record.model_visible_output, &call.name)?;
            runtime.append_message(ConversationMessage::tool(
                content,
                call.name.clone(),
                call.id.clone(),
            ));
            runtime.logger.log_tool_call(&ToolCallLogEntry {
                call,
                tool_group: None,
                result: ToolCallLogResult::Success {
                    output: record.model_visible_output.clone(),
                },
                latency_ms: started_at.elapsed().as_millis(),
            })?;
            runtime.logger.log_tool_activity(&ToolActivityLogEntry {
                tool_name,
                activity_verb,
                phase: ToolActivityPhase::Completed,
            })?;
            Ok(record)
        }
        Err(Error::Tool(error)) => {
            runtime.append_message(tool_error_message(&call, &error));
            runtime.logger.log_tool_call(&ToolCallLogEntry {
                call,
                tool_group: None,
                result: tool_error_log_result(&error),
                latency_ms: started_at.elapsed().as_millis(),
            })?;
            runtime.logger.log_tool_activity(&ToolActivityLogEntry {
                tool_name,
                activity_verb,
                phase: ToolActivityPhase::Completed,
            })?;
            Err(Error::Tool(error))
        }
        Err(error) => Err(error),
    }
}

async fn read_intrinsic_output(
    runtime: &ToolCallingRuntime,
    call: &ToolCall,
    operation_id: String,
) -> Result<ToolExecutionRecord> {
    let request = match serde_json::from_value::<ToolOutputReadRequest>(call.input.clone()) {
        Ok(request) => request,
        Err(source) => {
            return Err(Error::Tool(ToolError::invalid_arguments(
                TOOL_OUTPUT_READ,
                source,
            )));
        }
    };
    let output_id = request.output_id.clone();
    let window = match runtime
        .output_store()
        .read(ToolOutputStoreReadRequest {
            output_id: request.output_id,
            offset: request.offset,
            length: request.length,
            policy: runtime.output_policy(),
        })
        .await
    {
        Ok(window) => window,
        Err(error) => {
            return Err(Error::Tool(ToolError::execution(TOOL_OUTPUT_READ, error)));
        }
    };
    let envelope = readable_envelope(window, output_id.clone())?;
    let raw_output = envelope_value(&envelope, TOOL_OUTPUT_READ)?;
    Ok(ToolExecutionRecord {
        call_id: call.id.clone(),
        operation_id,
        name: call.name.clone(),
        output_id: Some(output_id),
        raw_output,
        model_visible_output: envelope,
    })
}
