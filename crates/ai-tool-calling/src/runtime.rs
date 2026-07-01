//! Retained runtime state and public mutation APIs.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, DynLogger, DynModel, DynTool, ToolDefinition};
use parking_lot::Mutex;

use crate::{Error, Result, turn::ActiveTurn};

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
    /// Builds a runtime from injected model, logger, and tool dependencies.
    pub fn new(
        system_prompt: impl Into<String>,
        model: DynModel,
        logger: DynLogger,
        tools: Vec<DynTool>,
    ) -> Result<Self> {
        let tools = ToolCatalog::new(tools)?;
        Ok(Self {
            model,
            logger,
            state: Arc::new(Mutex::new(RuntimeConfig {
                system_prompt: system_prompt.into(),
                conversation: Vec::new(),
                tools,
            })),
        })
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
}
