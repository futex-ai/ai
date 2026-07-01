//! Tool dispatch helper for active turns.

use std::time::Instant;

use ai_interface::{
    ConversationMessage, ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult, ToolError, ToolInvocation,
};

use crate::{Error, Result, ToolCallingRuntime};

use super::{
    failures::{tool_error_log_result, tool_error_message},
    types::ToolExecutionRecord,
};

pub(super) async fn dispatch_tool_call(
    runtime: &ToolCallingRuntime,
    call: ToolCall,
) -> Result<ToolExecutionRecord> {
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
        Ok(output) => {
            let record = ToolExecutionRecord {
                call_id: call.id.clone(),
                operation_id,
                name: call.name.clone(),
                output: output.clone(),
            };
            runtime.append_message(ConversationMessage::tool(
                output.to_string(),
                call.name.clone(),
                call.id.clone(),
            ));
            runtime.logger.log_tool_call(&ToolCallLogEntry {
                call,
                tool_group,
                result: ToolCallLogResult::Success { output },
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
