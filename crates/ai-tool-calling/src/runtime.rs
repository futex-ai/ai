//! Agent dependencies and configuration snapshots for caller-owned turns.

use std::{num::NonZeroUsize, sync::Arc};

use ai_interface::{ConversationMessage, DynLogger, DynModel, DynTool, ToolDefinition};
use parking_lot::Mutex;

use crate::{
    Agent, AgentRequest, DynAgentEventStream, DynToolOutputStore, Error, ModelResponseCheckpoint,
    Result, ToolOutputPolicy, ToolOutputPolicyLimits, TurnCheckpoint,
};
use crate::{catalog::ToolCatalog, turn::stream::TurnStream};

#[derive(Clone)]
pub(crate) struct RuntimeConfig {
    pub(crate) output_policy: ToolOutputPolicy,
    pub(crate) output_store: DynToolOutputStore,
    pub(crate) tools: ToolCatalog,
    pub(crate) tool_call_limit: usize,
}

#[derive(Clone)]
/// Reusable agent configuration with no retained conversation state.
pub struct ToolCallingRuntime {
    pub(crate) model: DynModel,
    pub(crate) logger: DynLogger,
    state: Arc<Mutex<RuntimeConfig>>,
}

impl ToolCallingRuntime {
    /// Builds an agent from model, observer, tools, and output dependencies.
    pub fn new(
        model: DynModel,
        logger: DynLogger,
        tools: Vec<DynTool>,
        output_store: DynToolOutputStore,
        output_policy: ToolOutputPolicy,
    ) -> Result<Self> {
        Ok(Self {
            model,
            logger,
            state: Arc::new(Mutex::new(RuntimeConfig {
                output_policy,
                output_store,
                tools: ToolCatalog::new(tools)?,
                tool_call_limit: 4,
            })),
        })
    }

    /// Builds an agent after validating raw output policy limits.
    pub fn new_with_output_policy_limits(
        model: DynModel,
        logger: DynLogger,
        tools: Vec<DynTool>,
        output_store: DynToolOutputStore,
        output_limits: ToolOutputPolicyLimits,
    ) -> Result<Self> {
        let output_policy = match ToolOutputPolicy::from_limits(output_limits) {
            Ok(policy) => policy,
            Err(source) => return Err(Error::OutputPolicy { source }),
        };
        Self::new(model, logger, tools, output_store, output_policy)
    }

    /// Sets the maximum number of simultaneous tool calls in future turns.
    pub fn with_tool_call_limit(self, limit: NonZeroUsize) -> Self {
        self.state.lock().tool_call_limit = limit.get();
        self
    }

    /// Returns ordinary tool definitions without the conditional output reader.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.state.lock().tools.definitions.clone()
    }

    /// Replaces the validated tool catalog for future turns.
    pub fn replace_tools(&self, tools: Vec<DynTool>) -> Result<()> {
        let tools = ToolCatalog::new(tools)?;
        self.state.lock().tools = tools;
        Ok(())
    }

    /// Replaces the output store for future turns.
    pub fn replace_output_store(&self, output_store: DynToolOutputStore) {
        self.state.lock().output_store = output_store;
    }

    pub(crate) fn snapshot(&self) -> RuntimeConfig {
        self.state.lock().clone()
    }
}

impl Agent for ToolCallingRuntime {
    fn run<'a>(
        &'a self,
        conversation: &'a mut Vec<ConversationMessage>,
        request: AgentRequest,
    ) -> DynAgentEventStream<'a> {
        Box::new(TurnStream::new(self, conversation, request, None, None))
    }

    fn run_with_checkpoints<'a>(
        &'a self,
        conversation: &'a mut Vec<ConversationMessage>,
        request: AgentRequest,
        checkpoint: &'a mut dyn TurnCheckpoint,
        response_checkpoint: &'a mut dyn ModelResponseCheckpoint,
    ) -> DynAgentEventStream<'a> {
        Box::new(TurnStream::new(
            self,
            conversation,
            request,
            Some(checkpoint),
            Some(response_checkpoint),
        ))
    }
}
