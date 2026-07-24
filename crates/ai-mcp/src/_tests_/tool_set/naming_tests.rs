//! Adapter naming and definition projection tests.

use ai_interface::Tool;
use serde_json::json;

use crate::{Error, McpServerConfig, McpToolDescriptor, McpToolSet};

use super::support::{descriptor, unused_client};

#[test]
fn sanitizes_truncates_and_suffixes_collisions_deterministically() {
    let long = "very-long-tool-name".repeat(8);
    let descriptors = vec![
        descriptor("a.b"),
        descriptor("a b"),
        descriptor(&long),
        descriptor(&long),
    ];

    let set = McpToolSet::new(
        unused_client(),
        &McpServerConfig::new("server", "https://example.com/mcp"),
        descriptors,
    )
    .unwrap();
    let names = set
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();

    assert_eq!(names[0], "mcp__server__a_b");
    assert_eq!(names[1], "mcp__server__a_b_2");
    assert_eq!(names[2].len(), 64);
    assert_eq!(names[3].len(), 64);
    assert!(names[3].ends_with("_2"));
}

#[test]
fn maps_description_title_schema_activity_and_group() {
    let descriptors = vec![
        McpToolDescriptor {
            name: "described".to_owned(),
            title: Some("Title".to_owned()),
            description: Some("Description".to_owned()),
            input_schema: json!({"type":"string"}),
            output_schema: None,
        },
        McpToolDescriptor {
            name: "titled".to_owned(),
            title: Some("Title only".to_owned()),
            description: None,
            input_schema: json!({"type":"number"}),
            output_schema: None,
        },
        descriptor("fallback"),
    ];
    let mut config = McpServerConfig::new("server", "https://example.com/mcp");
    config.activity_verb = Some("Searching".to_owned());

    let set = McpToolSet::new(unused_client(), &config, descriptors).unwrap();
    let definitions = set.definitions();

    assert_eq!(definitions[0].description, "Description");
    assert_eq!(definitions[1].description, "Title only");
    assert_eq!(definitions[2].description, "fallback");
    assert_eq!(definitions[0].input_schema, json!({"type":"string"}));
    assert_eq!(definitions[0].activity_verb.as_deref(), Some("Searching"));
    assert_eq!(set.group_for_tool(&definitions[0].name), Some("mcp"));
    assert_eq!(set.group_for_tool("not-owned"), None);
}

#[test]
fn rejects_invalid_server_keys_at_adapter_construction() {
    let result = McpToolSet::new(
        unused_client(),
        &McpServerConfig::new("Invalid", "https://example.com/mcp"),
        vec![descriptor("tool")],
    );

    assert!(matches!(result, Err(Error::InvalidServerKey { .. })));
}
