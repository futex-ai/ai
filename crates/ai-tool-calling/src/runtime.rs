//! Retained runtime state and public mutation APIs.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, DynLogger, DynModel, DynTool, ToolDefinition};
use parking_lot::Mutex;

use crate::intrinsic::{is_intrinsic_tool, tool_output_read_definition};
use crate::{
    DynToolOutputStore, Error, Result, ToolOutputPolicy, ToolOutputPolicyLimits, turn::ActiveTurn,
};

#[derive(Clone)]
struct ToolCatalog {
    activity_verbs_by_name: BTreeMap<String, String>,
    definitions: Vec<ToolDefinition>,
    groups_by_name: BTreeMap<String, String>,
    owners_by_name: BTreeMap<String, usize>,
    tools: Vec<DynTool>,
}

impl ToolCatalog {
    fn new(tools: Vec<DynTool>) -> Result<Self> {
        let mut activity_verbs_by_name = BTreeMap::new();
        let mut definitions = Vec::new();
        let mut groups_by_name = BTreeMap::new();
        let mut owners_by_name = BTreeMap::new();
        for (index, tool) in tools.iter().enumerate() {
            for definition in tool.definitions() {
                if is_intrinsic_tool(&definition.name) {
                    return Err(Error::ReservedToolDefinition {
                        name: definition.name,
                    });
                }
                if owners_by_name
                    .insert(definition.name.clone(), index)
                    .is_some()
                {
                    return Err(Error::DuplicateToolDefinition {
                        name: definition.name,
                    });
                }
                if let Some(activity_verb) = &definition.activity_verb {
                    activity_verbs_by_name.insert(definition.name.clone(), activity_verb.clone());
                }
                if let Some(group) = tool.group_for_tool(&definition.name) {
                    groups_by_name.insert(definition.name.clone(), group.to_owned());
                }
                definitions.push(definition);
            }
        }
        let intrinsic = tool_output_read_definition();
        if let Some(activity_verb) = &intrinsic.activity_verb {
            activity_verbs_by_name.insert(intrinsic.name.clone(), activity_verb.clone());
        }
        definitions.push(intrinsic);
        Ok(Self {
            activity_verbs_by_name,
            definitions,
            groups_by_name,
            owners_by_name,
            tools,
        })
    }
}

#[derive(Clone)]
struct RuntimeConfig {
    system_prompt: String,
    conversation: Vec<ConversationMessage>,
    output_policy: ToolOutputPolicy,
    output_store: DynToolOutputStore,
    tools: ToolCatalog,
}

#[derive(Clone)]
/// Stateful in-memory tool-calling runtime.
pub struct ToolCallingRuntime {
    pub(crate) model: DynModel,
    pub(crate) logger: DynLogger,
    state: Arc<Mutex<RuntimeConfig>>,
}

impl ToolCallingRuntime {
    /// Builds a runtime from injected model, logger, tool, and output-store dependencies.
    pub fn new(
        system_prompt: impl Into<String>,
        model: DynModel,
        logger: DynLogger,
        tools: Vec<DynTool>,
        output_store: DynToolOutputStore,
        output_policy: ToolOutputPolicy,
    ) -> Result<Self> {
        let tools = ToolCatalog::new(tools)?;
        Ok(Self {
            model,
            logger,
            state: Arc::new(Mutex::new(RuntimeConfig {
                system_prompt: system_prompt.into(),
                conversation: Vec::new(),
                output_policy,
                output_store,
                tools,
            })),
        })
    }

    /// Builds a runtime after validating raw output policy limit values.
    pub fn new_with_output_policy_limits(
        system_prompt: impl Into<String>,
        model: DynModel,
        logger: DynLogger,
        tools: Vec<DynTool>,
        output_store: DynToolOutputStore,
        output_limits: ToolOutputPolicyLimits,
    ) -> Result<Self> {
        let output_policy = match ToolOutputPolicy::from_limits(output_limits) {
            Ok(output_policy) => output_policy,
            Err(source) => return Err(Error::OutputPolicy { source }),
        };
        Self::new(
            system_prompt,
            model,
            logger,
            tools,
            output_store,
            output_policy,
        )
    }

    /// Returns the retained conversation state.
    pub fn conversation(&self) -> Vec<ConversationMessage> {
        self.state.lock().conversation.clone()
    }

    /// Replaces the retained conversation state.
    pub fn replace_conversation(&self, messages: Vec<ConversationMessage>) {
        self.state.lock().conversation = messages;
    }

    /// Appends one caller/user message.
    pub fn push_user_message(&self, content: impl Into<String>) {
        self.state
            .lock()
            .conversation
            .push(ConversationMessage::user(content));
    }

    /// Appends one assistant message.
    pub fn push_assistant_message(&self, content: impl Into<String>) {
        self.state
            .lock()
            .conversation
            .push(ConversationMessage::assistant(content, Vec::new()));
    }

    /// Clears the retained conversation state.
    pub fn clear_conversation(&self) {
        self.state.lock().conversation.clear();
    }

    /// Returns the active system prompt.
    pub fn system_prompt(&self) -> String {
        self.state.lock().system_prompt.clone()
    }

    /// Replaces the active system prompt.
    pub fn set_system_prompt(&self, system_prompt: impl Into<String>) {
        self.state.lock().system_prompt = system_prompt.into();
    }

    /// Returns the currently exposed tool definitions.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.state.lock().tools.definitions.clone()
    }

    /// Replaces the active tools for future turns.
    pub fn replace_tools(&self, tools: Vec<DynTool>) -> Result<()> {
        self.state.lock().tools = ToolCatalog::new(tools)?;
        Ok(())
    }

    /// Replaces the output store for future tool output reads and writes.
    pub fn replace_output_store(&self, output_store: DynToolOutputStore) {
        self.state.lock().output_store = output_store;
    }

    /// Appends a caller-supplied message and starts a new turn handle.
    pub fn send<'a>(
        &'a self,
        message: ConversationMessage,
        max_steps: Option<usize>,
    ) -> ActiveTurn<'a> {
        self.state.lock().conversation.push(message);
        ActiveTurn::new(self, max_steps)
    }

    /// Starts a turn handle without appending another caller message.
    pub fn resume(&self, max_steps: Option<usize>) -> ActiveTurn<'_> {
        ActiveTurn::new(self, max_steps)
    }

    pub(crate) fn request_snapshot(
        &self,
    ) -> (String, Vec<ConversationMessage>, Vec<ToolDefinition>) {
        let state = self.state.lock();
        (
            state.system_prompt.clone(),
            state.conversation.clone(),
            state.tools.definitions.clone(),
        )
    }

    pub(crate) fn append_message(&self, message: ConversationMessage) {
        self.state.lock().conversation.push(message);
    }

    pub(crate) fn tool_for_name(&self, name: &str) -> Option<DynTool> {
        let state = self.state.lock();
        state
            .tools
            .owners_by_name
            .get(name)
            .and_then(|index| state.tools.tools.get(*index))
            .cloned()
    }

    pub(crate) fn activity_verb_for_name(&self, name: &str) -> Option<String> {
        self.state
            .lock()
            .tools
            .activity_verbs_by_name
            .get(name)
            .cloned()
    }

    pub(crate) fn tool_group_for_name(&self, name: &str) -> Option<String> {
        self.state.lock().tools.groups_by_name.get(name).cloned()
    }

    pub(crate) fn output_store(&self) -> DynToolOutputStore {
        self.state.lock().output_store.clone()
    }

    pub(crate) fn output_policy(&self) -> ToolOutputPolicy {
        self.state.lock().output_policy
    }
}
