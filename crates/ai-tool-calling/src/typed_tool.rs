//! Typed tool arguments, output serialization, and generated input schemas.

use std::io;

use ai_interface::{SafeErrorSource, Tool, ToolDefinition, ToolError, ToolInvocation, ToolResult};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[async_trait]
/// One tool whose argument schema comes from its Rust argument type.
pub trait TypedTool: Send + Sync {
    /// Owned argument type decoded before execution.
    type Args: DeserializeOwned + JsonSchema + Send;
    /// Successful output serialized to JSON before output management.
    type Output: Serialize + Send;

    /// Stable model-visible tool name.
    fn name(&self) -> &str;

    /// Description included with the input schema.
    fn description(&self) -> &str;

    /// Optional public activity label.
    fn activity_verb(&self) -> Option<&str> {
        None
    }

    /// Optional observer group of this tool.
    fn group(&self) -> Option<&'static str> {
        None
    }

    /// Executes valid arguments with the call's operation identifier.
    async fn execute(&self, args: Self::Args, operation_id: &str) -> ToolResult<Self::Output>;
}

/// Exposes a typed tool through the shared generic tool boundary.
pub struct TypedToolAdapter<T: TypedTool> {
    tool: T,
}

impl<T: TypedTool> TypedToolAdapter<T> {
    /// Wraps one typed tool for injection into an agent.
    pub fn new(tool: T) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl<T: TypedTool> Tool for TypedToolAdapter<T> {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: self.tool.name().to_owned(),
            description: self.tool.description().to_owned(),
            input_schema: schemars::schema_for!(T::Args).to_value(),
            activity_verb: self.tool.activity_verb().map(str::to_owned),
        }]
    }

    async fn call(&self, tool_name: &str, input: Value) -> ToolResult<Value> {
        self.call_with_invocation(ToolInvocation {
            tool_name: tool_name.to_owned(),
            input,
            operation_id: String::new(),
        })
        .await
    }

    async fn call_with_invocation(&self, invocation: ToolInvocation) -> ToolResult<Value> {
        if invocation.tool_name != self.tool.name() {
            return Err(ToolError::UnknownTool {
                name: invocation.tool_name,
            });
        }
        let args = match serde_json::from_value::<T::Args>(invocation.input) {
            Ok(args) => args,
            Err(source) => return Err(invalid_arguments(self.tool.name(), source)),
        };
        let output = match self.tool.execute(args, &invocation.operation_id).await {
            Ok(output) => output,
            Err(error) => return Err(safe_execution_error(error)),
        };
        match serde_json::to_value(output) {
            Ok(value) => Ok(value),
            Err(source) => Err(ToolError::execution(
                self.tool.name(),
                SafeErrorSource::redacted(source),
            )),
        }
    }

    fn group_for_tool(&self, tool_name: &str) -> Option<&'static str> {
        if tool_name == self.tool.name() {
            self.tool.group()
        } else {
            None
        }
    }
}

fn invalid_arguments(tool_name: &str, source: serde_json::Error) -> ToolError {
    ToolError::invalid_arguments(
        tool_name,
        serde_json::Error::io(io::Error::other(SafeErrorSource::redacted(source))),
    )
}

fn safe_execution_error(error: ToolError) -> ToolError {
    match error {
        ToolError::InvalidArguments { tool_name, source } => invalid_arguments(&tool_name, source),
        ToolError::Execution { ref tool_name, .. } => {
            ToolError::execution(tool_name.clone(), SafeErrorSource::redacted(error))
        }
        error => error,
    }
}
