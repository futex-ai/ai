//! Catalog construction and replacement use the same name rules.

use std::sync::Arc;

use ai_interface::{DynTool, NoopLogger, ToolDefinition, ToolMock};
use ai_tool_calling::{Error, InMemoryToolOutputStore, ToolCallingRuntime, ToolOutputPolicy};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

fn tool(names: &[&str]) -> DynTool {
    Arc::new(Unimock::new(
        ToolMock::definitions.each_call(matching!()).returns(
            names
                .iter()
                .map(|name| ToolDefinition {
                    name: (*name).to_owned(),
                    description: "Catalog tool.".to_owned(),
                    input_schema: json!({"type":"object"}),
                    activity_verb: None,
                })
                .collect::<Vec<_>>(),
        ),
    ))
}

fn build(tools: Vec<DynTool>) -> ai_tool_calling::Result<ToolCallingRuntime> {
    ToolCallingRuntime::new(
        Arc::new(Unimock::new(())),
        Arc::new(NoopLogger),
        tools,
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
}

#[test]
fn duplicate_names_fail_build_and_invalid_replacement_keeps_the_valid_catalog() {
    let result = build(vec![tool(&["duplicate"]), tool(&["duplicate"])]);
    assert!(matches!(result,Err(Error::DuplicateToolDefinition {name}) if name=="duplicate"));

    let agent = build(vec![tool(&["original"])]).unwrap();
    let result = agent.replace_tools(vec![tool(&["duplicate", "duplicate"])]);
    assert!(matches!(result,Err(Error::DuplicateToolDefinition {name}) if name=="duplicate"));
    assert_eq!(agent.tool_definitions()[0].name, "original");

    let result = agent.replace_tools(vec![tool(&["tool_output_read"])]);
    assert!(matches!(result,Err(Error::ReservedToolDefinition {name}) if name=="tool_output_read"));
    assert_eq!(agent.tool_definitions()[0].name, "original");

    agent.replace_tools(vec![tool(&["replacement"])]).unwrap();
    assert_eq!(agent.tool_definitions()[0].name, "replacement");
}
