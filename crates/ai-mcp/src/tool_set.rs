//! Immutable MCP tool snapshot exposed through `ai_interface::Tool`.

use std::collections::BTreeMap;

use ai_interface::{Tool, ToolDefinition, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

use crate::{
    DynMcpClient, McpServerConfig, McpToolDescriptor, Result, tool_set_naming::prefixed_names,
    tool_set_result::map_outcome,
};

/// Immutable adapter snapshot for one MCP server's discovered tools.
pub struct McpToolSet {
    client: DynMcpClient,
    definitions: Vec<ToolDefinition>,
    originals_by_name: BTreeMap<String, String>,
    max_response_bytes: usize,
}

impl McpToolSet {
    /// Builds an adapter from an already-discovered tool snapshot.
    pub fn new(
        client: DynMcpClient,
        config: &McpServerConfig,
        descriptors: Vec<McpToolDescriptor>,
    ) -> Result<Self> {
        config.validate()?;
        let names = prefixed_names(&config.server_key, &descriptors);
        let mut definitions = Vec::with_capacity(descriptors.len());
        let mut originals_by_name = BTreeMap::new();
        for (descriptor, prefixed_name) in descriptors.into_iter().zip(names) {
            let description = descriptor
                .description
                .or(descriptor.title)
                .unwrap_or_else(|| descriptor.name.clone());
            definitions.push(ToolDefinition {
                name: prefixed_name.clone(),
                description,
                input_schema: descriptor.input_schema,
                activity_verb: config.activity_verb.clone(),
            });
            originals_by_name.insert(prefixed_name, descriptor.name);
        }
        Ok(Self {
            client,
            definitions,
            originals_by_name,
            max_response_bytes: config.max_response_bytes,
        })
    }

    /// Discovers all tools and builds a new immutable adapter snapshot.
    pub async fn load(client: DynMcpClient, config: &McpServerConfig) -> Result<Self> {
        let descriptors = client.list_tools().await?;
        Self::new(client, config, descriptors)
    }
}

#[async_trait]
impl Tool for McpToolSet {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    async fn call(&self, tool_name: &str, input: Value) -> ToolResult<Value> {
        let Some(original_name) = self.originals_by_name.get(tool_name) else {
            return Err(ToolError::UnknownTool {
                name: tool_name.to_owned(),
            });
        };
        let outcome = match self.client.call_tool(original_name, input).await {
            Ok(outcome) => outcome,
            Err(source) => return Err(ToolError::execution(tool_name, source)),
        };
        map_outcome(tool_name, outcome, self.max_response_bytes)
    }

    fn group_for_tool(&self, tool_name: &str) -> Option<&'static str> {
        self.originals_by_name
            .contains_key(tool_name)
            .then_some("mcp")
    }
}
