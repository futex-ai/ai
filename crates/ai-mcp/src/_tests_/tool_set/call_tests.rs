//! Adapter dispatch and result-mapping tests.

use std::collections::BTreeMap;

use ai_interface::{Tool, ToolError};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, McpAnnotations, McpClientMock, McpContentBlock, McpRole, McpServerConfig,
    McpToolCallOutcome, McpToolSet,
};

use super::support::{client, descriptor, unused_client};

#[tokio::test]
async fn load_snapshots_descriptors_and_dispatches_to_original_name() {
    let mock = Unimock::new((
        McpClientMock::list_tools
            .next_call(matching!())
            .returns(Ok(vec![descriptor("original")])),
        McpClientMock::call_tool
            .next_call(matching!("original", _))
            .returns(Ok(text_outcome("done"))),
    ));
    let config = McpServerConfig::new("demo", "https://example.com/mcp");
    let set = McpToolSet::load(client(mock), &config).await.unwrap();

    let output = set
        .call("mcp__demo__original", json!({"value": 1}))
        .await
        .unwrap();

    assert_eq!(output, Value::String("done".to_owned()));
}

#[tokio::test]
async fn unknown_names_fail_before_client_dispatch() {
    let set = McpToolSet::new(
        unused_client(),
        &McpServerConfig::new("demo", "https://example.com/mcp"),
        vec![descriptor("known")],
    )
    .unwrap();

    let error = set.call("mcp__demo__missing", json!({})).await.unwrap_err();

    assert!(matches!(error, ToolError::UnknownTool { .. }));
}

#[tokio::test]
async fn protocol_failures_become_tool_execution_errors() {
    let mock = Unimock::new(
        McpClientMock::call_tool
            .next_call(matching!("original", _))
            .returns(Err(Error::Transport {
                message: "offline".to_owned(),
            })),
    );
    let set = McpToolSet::new(
        client(mock),
        &McpServerConfig::new("demo", "https://example.com/mcp"),
        vec![descriptor("original")],
    )
    .unwrap();

    let error = set
        .call("mcp__demo__original", json!({}))
        .await
        .unwrap_err();

    assert!(matches!(error, ToolError::Execution { .. }));
}

#[tokio::test]
async fn applies_result_precedence_and_preserves_wire_content() {
    let annotated = McpContentBlock::Text {
        text: "failed".to_owned(),
        annotations: Some(McpAnnotations {
            audience: Some(vec![McpRole::Assistant]),
            priority: None,
            last_modified: None,
        }),
        meta: Some(BTreeMap::from([("trace".to_owned(), json!("one"))])),
    };
    let error_output = call_once(McpToolCallOutcome {
        content: vec![annotated.clone()],
        structured_content: Some(json!({"ignored": true})),
        is_error: true,
    })
    .await;
    let structured = call_once(McpToolCallOutcome {
        content: vec![annotated.clone()],
        structured_content: Some(json!({"answer": 42})),
        is_error: false,
    })
    .await;
    let multi = call_once(McpToolCallOutcome {
        content: vec![
            annotated,
            McpContentBlock::Unknown(json!({"type":"future","value":7})),
        ],
        structured_content: None,
        is_error: false,
    })
    .await;

    assert_eq!(error_output["is_error"], true);
    assert_eq!(error_output["content"][0]["_meta"]["trace"], "one");
    assert_eq!(structured, json!({"answer": 42}));
    assert_eq!(multi[1], json!({"type":"future","value":7}));
}

#[tokio::test]
async fn truncation_envelope_ends_on_a_utf8_boundary() {
    let mut config = McpServerConfig::new("demo", "https://example.com/mcp");
    config.max_response_bytes = 80;
    let output = call_once_with_config(text_outcome(&"é".repeat(100)), config).await;

    assert_eq!(output["truncated"], true);
    assert!(output["content"].as_str().is_some());
    assert!(serde_json::to_vec(&output).unwrap().len() <= 80);
}

async fn call_once(outcome: McpToolCallOutcome) -> Value {
    call_once_with_config(
        outcome,
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .await
}

async fn call_once_with_config(outcome: McpToolCallOutcome, config: McpServerConfig) -> Value {
    let mock = Unimock::new(
        McpClientMock::call_tool
            .next_call(matching!("original", _))
            .returns(Ok(outcome)),
    );
    let set = McpToolSet::new(client(mock), &config, vec![descriptor("original")]).unwrap();
    set.call("mcp__demo__original", json!({})).await.unwrap()
}

fn text_outcome(text: &str) -> McpToolCallOutcome {
    McpToolCallOutcome {
        content: vec![McpContentBlock::Text {
            text: text.to_owned(),
            annotations: None,
            meta: None,
        }],
        structured_content: None,
        is_error: false,
    }
}
