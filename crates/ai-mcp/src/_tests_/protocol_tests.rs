//! MCP wire-schema tests.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{
    McpAnnotations, McpContentBlock, McpResourceContents, McpRole, McpToolCallOutcome,
    protocol::{InitializeResult, ListToolsResult},
};

#[test]
fn decodes_every_known_content_block_and_wire_name() {
    let blocks: Vec<McpContentBlock> = serde_json::from_value(json!([
        {
            "type": "text",
            "text": "hello",
            "annotations": {
                "audience": ["user", "assistant"],
                "priority": 1,
                "lastModified": "2026-07-24T10:00:00Z"
            },
            "_meta": {"trace": "one"}
        },
        {"type": "image", "data": "aW1n", "mimeType": "image/png"},
        {"type": "audio", "data": "YXVkaW8=", "mimeType": "audio/wav"},
        {
            "type": "resource_link",
            "name": "guide",
            "title": "Guide",
            "uri": "file:///guide.md",
            "description": "Read me",
            "mimeType": "text/markdown",
            "size": 42
        },
        {
            "type": "resource",
            "resource": {
                "uri": "file:///note.txt",
                "mimeType": "text/plain",
                "text": "note",
                "_meta": {"source": "test"}
            }
        }
    ]))
    .unwrap();

    assert!(matches!(
        &blocks[0],
        McpContentBlock::Text {
            annotations: Some(McpAnnotations {
                audience: Some(audience),
                ..
            }),
            ..
        } if audience == &[McpRole::User, McpRole::Assistant]
    ));
    assert!(
        matches!(&blocks[1], McpContentBlock::Image { mime_type, .. } if mime_type == "image/png")
    );
    assert!(
        matches!(&blocks[2], McpContentBlock::Audio { mime_type, .. } if mime_type == "audio/wav")
    );
    assert!(
        matches!(&blocks[3], McpContentBlock::ResourceLink { size: Some(size), .. } if size.as_u64() == Some(42))
    );
    assert!(matches!(
        &blocks[4],
        McpContentBlock::EmbeddedResource {
            resource: McpResourceContents::Text { text, .. },
            ..
        } if text == "note"
    ));
}

#[test]
fn preserves_unknown_content_exactly_on_round_trip() {
    let original = json!({
        "type": "future",
        "nested": {"anything": [1, true, null]},
        "_meta": {"version": 2}
    });
    let block: McpContentBlock = serde_json::from_value(original.clone()).unwrap();

    assert_eq!(block, McpContentBlock::Unknown(original.clone()));
    assert_eq!(serde_json::to_value(block).unwrap(), original);
}

#[test]
fn rejects_malformed_known_blocks_and_ambiguous_resources() {
    assert!(
        serde_json::from_value::<McpContentBlock>(json!({"type": "image", "data": "x"})).is_err()
    );
    assert!(
        serde_json::from_value::<McpContentBlock>(json!({
            "type": "resource",
            "resource": {"uri": "file:///x", "text": "x", "blob": "eA=="}
        }))
        .is_err()
    );
}

#[test]
fn call_result_defaults_is_error_and_uses_camel_case() {
    let result: McpToolCallOutcome = serde_json::from_value(json!({
        "content": [{"type": "text", "text": "ok"}],
        "structuredContent": {"answer": 42}
    }))
    .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.structured_content, Some(json!({"answer": 42})));
}

#[test]
fn initialize_and_tool_list_project_wire_defaults() {
    let result: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "demo", "title": "Demo", "version": "1.0"},
        "instructions": "Use carefully"
    }))
    .unwrap();
    let list: ListToolsResult = serde_json::from_value(json!({
        "tools": [{
            "name": "lookup",
            "description": "Looks up a value",
            "inputSchema": {"type": "object"},
            "outputSchema": {"type": "string"}
        }],
        "nextCursor": "page-2"
    }))
    .unwrap();

    assert!(!result.capabilities.tools.unwrap().list_changed);
    assert_eq!(list.tools[0].input_schema, json!({"type": "object"}));
    assert_eq!(list.next_cursor.as_deref(), Some("page-2"));
}

#[test]
fn content_serialization_retains_annotations_and_metadata() {
    let block = McpContentBlock::Text {
        text: "hello".to_owned(),
        annotations: Some(McpAnnotations {
            audience: Some(vec![McpRole::Assistant]),
            priority: serde_json::Number::from_f64(0.5),
            last_modified: Some("now".to_owned()),
        }),
        meta: Some(BTreeMap::from([("source".to_owned(), Value::Bool(true))])),
    };
    let value = serde_json::to_value(block).unwrap();

    assert_eq!(value["annotations"]["lastModified"], "now");
    assert_eq!(value["_meta"]["source"], true);
}
