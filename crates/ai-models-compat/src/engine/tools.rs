//! Shared function-tool and tool-call wire mapping.

use super::request_types::{
    ChatCompletionsTool, ChatCompletionsToolCall, ChatCompletionsToolDefinition,
    ChatCompletionsToolFunction,
};
use ai_interface::{ToolCall, ToolDefinition};

pub(super) fn tool_call(call: &ToolCall) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function",
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.input.to_string(),
        },
    }
}

pub(super) fn raw_tool_call(id: &str, name: &str, arguments: &str) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: id.to_owned(),
        kind: "function",
        function: ChatCompletionsToolFunction {
            name: name.to_owned(),
            arguments: arguments.to_owned(),
        },
    }
}

pub(super) fn tool(tool: &ToolDefinition) -> ChatCompletionsTool {
    ChatCompletionsTool {
        kind: "function",
        function: ChatCompletionsToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        },
    }
}
