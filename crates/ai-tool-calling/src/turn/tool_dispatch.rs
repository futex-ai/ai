//! Dispatches one call against the immutable dependencies of a turn.

use ai_interface::{SafeErrorSource, ToolCall, ToolError, ToolInvocation, ToolOutputReadRequest};

use crate::intrinsic::is_intrinsic_tool;
use crate::runtime::RuntimeConfig;
use crate::tool_output::{
    TOOL_OUTPUT_READ, envelope_value, manage_successful_tool_output, readable_envelope,
};
use crate::{Error, Result, ToolExecutionRecord, ToolOutputStoreReadRequest};

pub(super) async fn dispatch_tool_call(
    config: RuntimeConfig,
    call: ToolCall,
) -> Result<ToolExecutionRecord> {
    let operation_id = call.operation_id.clone().unwrap_or_else(|| call.id.clone());
    if is_intrinsic_tool(&call.name) {
        return read_output(&config, &call, operation_id).await;
    }
    let Some(tool) = config.tools.tool_for_name(&call.name) else {
        return Err(Error::Tool(ToolError::UnknownTool { name: call.name }));
    };
    let raw_output = tool
        .call_with_invocation(ToolInvocation {
            tool_name: call.name.clone(),
            input: call.input.clone(),
            operation_id: operation_id.clone(),
        })
        .await?;
    let managed = manage_successful_tool_output(&config, &call.name, raw_output).await?;
    Ok(ToolExecutionRecord {
        call_id: call.id,
        operation_id,
        name: call.name,
        output_id: managed.output_id,
        raw_output: managed.raw_output,
        model_visible_output: managed.model_visible_output,
    })
}

async fn read_output(
    config: &RuntimeConfig,
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
    let window = match config
        .output_store
        .read(ToolOutputStoreReadRequest {
            output_id: request.output_id,
            offset: request.offset,
            length: request.length,
            policy: config.output_policy,
        })
        .await
    {
        Ok(window) => window,
        Err(source) => {
            return Err(Error::Tool(ToolError::execution(
                TOOL_OUTPUT_READ,
                SafeErrorSource::redacted(source),
            )));
        }
    };
    let envelope = readable_envelope(window, output_id.clone())?;
    Ok(ToolExecutionRecord {
        call_id: call.id.clone(),
        operation_id,
        name: call.name.clone(),
        output_id: Some(output_id),
        raw_output: envelope_value(&envelope, TOOL_OUTPUT_READ)?,
        model_visible_output: envelope,
    })
}
