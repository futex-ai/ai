//! MCP content serialization regressions.

use serde_json::json;

use crate::McpContentBlock;

#[test]
fn partial_annotations_round_trip_without_absent_members() {
    let original = json!({
        "type": "text",
        "text": "hello",
        "annotations": {
            "audience": ["assistant"]
        }
    });
    let block: McpContentBlock = serde_json::from_value(original.clone()).unwrap();

    assert_eq!(serde_json::to_value(block).unwrap(), original);
}

#[test]
fn present_empty_and_zero_annotation_values_are_retained() {
    let block: McpContentBlock = serde_json::from_value(json!({
        "type": "text",
        "text": "hello",
        "annotations": {
            "audience": [],
            "priority": 0,
            "lastModified": null
        }
    }))
    .unwrap();
    let value = serde_json::to_value(block).unwrap();

    assert_eq!(
        value["annotations"],
        json!({
            "audience": [],
            "priority": 0
        })
    );
}
