//! Validated ordinary tool definitions and their dispatch metadata.

use std::collections::BTreeMap;

use ai_interface::{DynTool, ToolDefinition};

use crate::intrinsic::is_intrinsic_tool;
use crate::{Error, Result};

#[derive(Clone)]
pub(crate) struct ToolCatalog {
    pub(crate) activity_verbs_by_name: BTreeMap<String, String>,
    pub(crate) definitions: Vec<ToolDefinition>,
    pub(crate) groups_by_name: BTreeMap<String, String>,
    owners_by_name: BTreeMap<String, usize>,
    tools: Vec<DynTool>,
}

impl ToolCatalog {
    pub(crate) fn new(tools: Vec<DynTool>) -> Result<Self> {
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
        Ok(Self {
            activity_verbs_by_name,
            definitions,
            groups_by_name,
            owners_by_name,
            tools,
        })
    }

    pub(crate) fn tool_for_name(&self, name: &str) -> Option<DynTool> {
        self.owners_by_name
            .get(name)
            .and_then(|index| self.tools.get(*index))
            .cloned()
    }
}
