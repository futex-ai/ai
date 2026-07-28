//! Tool-list pagination bound and cursor-cycle tests.

use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicBool},
};

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpClient, McpServerConfig, StreamableHttpMcpClient};

use super::{
    lifecycle_tests::initialized_response,
    support::{ScriptedTransport, empty_response, event_response, json_response},
};

#[tokio::test]
async fn repeated_cursor_stops_before_duplicate_request() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        tool_page(2, "one", Some("same")),
        tool_page(3, "two", Some("same")),
    ]);
    let client = client(transport.clone(), config(2));

    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(
        error,
        Error::PaginationCursorCycle { method } if method == "tools/list"
    ));
    assert_eq!(transport.posts().len(), 4);
}

#[tokio::test]
async fn rejects_longer_cursor_cycles_before_reissuing_a_cursor() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        tool_page(2, "one", Some("a")),
        tool_page(3, "two", Some("b")),
        tool_page(4, "three", Some("a")),
    ]);
    let client = client(transport.clone(), config(100));

    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(error, Error::PaginationCursorCycle { .. }));
    assert_eq!(transport.posts().len(), 5);
}

#[tokio::test]
async fn rejects_distinct_cursors_before_exceeding_the_page_limit() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        tool_page(2, "one", Some("a")),
        tool_page(3, "two", Some("b")),
        tool_page(4, "three", None),
    ]);
    let client = client(transport.clone(), config(2));

    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(
        error,
        Error::PaginationLimitExceeded {
            method,
            limit: 2
        } if method == "tools/list"
    ));
    assert_eq!(transport.posts().len(), 4);
}

#[tokio::test]
async fn one_page_limit_stops_after_the_initial_page() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        tool_page(2, "one", Some("a")),
        tool_page(3, "two", None),
    ]);
    let client = client(transport.clone(), config(1));

    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(
        error,
        Error::PaginationLimitExceeded { limit: 1, .. }
    ));
    assert_eq!(transport.posts().len(), 3);
}

#[tokio::test]
async fn accepts_an_empty_cursor_once() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        tool_page(2, "one", Some("")),
        tool_page(3, "two", None),
    ]);
    let client = client(transport.clone(), config(2));

    let tools = client.list_tools().await.unwrap();

    assert_eq!(
        tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert_eq!(transport.posts()[3].body["params"]["cursor"], "");
}

#[tokio::test]
async fn pagination_failure_preserves_tool_list_invalidation() {
    let gate = Arc::new(AtomicBool::new(true));
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        event_response(
            vec![
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/tools/list_changed"
                }),
                tool_page_body(2, "one", Some("same")),
            ],
            gate,
        ),
        tool_page(3, "two", Some("same")),
    ]);
    let client = client(transport, config(2));

    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(error, Error::PaginationCursorCycle { .. }));
    assert!(client.tools_list_changed());
}

fn client(transport: Arc<ScriptedTransport>, config: McpServerConfig) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(transport, Arc::new(StaticHeaderAuth::default()), config).unwrap()
}

fn tool_page(id: u64, name: &str, next_cursor: Option<&str>) -> crate::McpHttpResponse {
    json_response(200, tool_page_body(id, name, next_cursor), BTreeMap::new())
}

fn tool_page_body(id: u64, name: &str, next_cursor: Option<&str>) -> serde_json::Value {
    let mut result = json!({
        "tools": [{"name": name, "inputSchema": {"type": "object"}}]
    });
    if let Some(cursor) = next_cursor {
        result["nextCursor"] = json!(cursor);
    }
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn config(max_tool_pages: usize) -> McpServerConfig {
    let mut config = McpServerConfig::new("demo", endpoint());
    config.max_tool_pages = max_tool_pages;
    config
}

fn endpoint() -> &'static str {
    "https://example.com/mcp"
}
