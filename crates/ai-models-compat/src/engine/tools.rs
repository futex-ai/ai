//! Shared function-tool and tool-call wire mapping.

use super::request_types::{
    ChatCompletionsTool, ChatCompletionsToolCall, ChatCompletionsToolDefinition,
    ChatCompletionsToolFunction,
};
use ai_interface::{DeepSeekToolCallContext, ToolCall, ToolDefinition};

pub(super) fn tool_call(call: &ToolCall) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.input.to_string(),
        },
    }
}

pub(super) fn raw_tool_call(call: &DeepSeekToolCallContext) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        },
    }
}

pub(super) fn tool(tool: &ToolDefinition) -> ChatCompletionsTool {
    ChatCompletionsTool {
        kind: "function".to_owned(),
        function: ChatCompletionsToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        },
    }
}
