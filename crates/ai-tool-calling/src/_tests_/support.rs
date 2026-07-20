use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{
    ConversationMessage, ConversationRole, DynLogger, DynModel, DynTool, NoopLogger,
    ToolDefinition, ToolError, ToolMock,
};
use serde::Deserialize;
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{InMemoryToolOutputStore, ToolCallingRuntime, ToolOutputPolicy};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EchoArgs {
    pub(crate) message: String,
}

#[derive(Clone)]
pub(crate) struct TypedEchoTool {
    parse_count: Arc<AtomicUsize>,
    tool: DynTool,
}

impl TypedEchoTool {
    pub(crate) fn succeeding() -> Self {
        Self::build(0)
    }

    pub(crate) fn fail_once() -> Self {
        Self::build(1)
    }

    pub(crate) fn parse_count(&self) -> usize {
        self.parse_count.load(Ordering::SeqCst)
    }

    pub(crate) fn tool(&self) -> DynTool {
        self.tool.clone()
    }
}

impl TypedEchoTool {
    fn build(failures: usize) -> Self {
        let parse_count = Arc::new(AtomicUsize::new(0));
        let execution_failures = Arc::new(AtomicUsize::new(failures));
        let tool = Arc::new(Unimock::new((
            ToolMock::definitions
                .each_call(matching!())
                .returns(vec![ToolDefinition {
                    name: "echo".to_owned(),
                    description: "Echo a typed message.".to_owned(),
                    input_schema: json!({
                        "type": "object",
                        "required": ["message"],
                        "properties": {
                            "message": { "type": "string" }
                        }
                    }),
                    activity_verb: Some("Echoing".to_owned()),
                }]),
            ToolMock::call.each_call(matching!("echo", _)).answers_arc({
                let parse_count = parse_count.clone();
                let execution_failures = execution_failures.clone();
                Arc::new(move |_, tool_name: &str, input: Value| {
                    let request: EchoArgs = serde_json::from_value(input)
                        .map_err(|source| ToolError::invalid_arguments(tool_name, source))?;
                    parse_count.fetch_add(1, Ordering::SeqCst);
                    if execution_failures.load(Ordering::SeqCst) > 0 {
                        execution_failures.fetch_sub(1, Ordering::SeqCst);
                        return Err(ToolError::execution(tool_name, FixtureToolError));
                    }
                    Ok(json!({ "echo": request.message }))
                })
            }),
        )));
        Self { parse_count, tool }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("[ai_tool_calling/tests] typed tool execution failed")]
struct FixtureToolError;

pub(crate) fn runtime(model: DynModel, tools: Vec<DynTool>) -> crate::Result<ToolCallingRuntime> {
    runtime_with_logger(model, Arc::new(NoopLogger), tools)
}

pub(crate) fn runtime_with_store_and_policy(
    model: DynModel,
    tools: Vec<DynTool>,
    output_store: crate::DynToolOutputStore,
    output_policy: ToolOutputPolicy,
) -> crate::Result<ToolCallingRuntime> {
    runtime_with_logger_store_and_policy(
        model,
        Arc::new(NoopLogger),
        tools,
        output_store,
        output_policy,
    )
}

pub(crate) fn runtime_with_logger(
    model: DynModel,
    logger: DynLogger,
    tools: Vec<DynTool>,
) -> crate::Result<ToolCallingRuntime> {
    runtime_with_logger_store_and_policy(
        model,
        logger,
        tools,
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
}

pub(crate) fn runtime_with_logger_store_and_policy(
    model: DynModel,
    logger: DynLogger,
    tools: Vec<DynTool>,
    output_store: crate::DynToolOutputStore,
    output_policy: ToolOutputPolicy,
) -> crate::Result<ToolCallingRuntime> {
    ToolCallingRuntime::new(
        "system prompt",
        model,
        logger,
        tools,
        output_store,
        output_policy,
    )
}

pub(crate) fn user_message(content: &str) -> ConversationMessage {
    ConversationMessage {
        role: ConversationRole::User,
        content: content.to_owned(),
        content_parts: Vec::new(),
        name: None,
        tool_call_id: None,
        tool_calls: Vec::new(),
        provider_context: Vec::new(),
    }
}
