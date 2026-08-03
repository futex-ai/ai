//! In-band tool-list invalidation tests.

use std::sync::{Arc, atomic::AtomicBool};

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{McpClient, McpServerConfig, StreamableHttpMcpClient};

use super::super::{
    lifecycle_tests::initialized_response,
    support::{ScriptedTransport, empty_response, event_response},
};

#[tokio::test]
async fn invalidation_during_successful_list_remains_stale() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        list_with_invalidation(2),
    ]);
    let client = client(transport);

    client.list_tools().await.unwrap();

    assert!(client.tools_list_changed());
}

#[tokio::test]
async fn second_invalidation_during_list_is_not_hidden_by_prior_staleness() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        call_with_invalidation(2),
        list_with_invalidation(3),
    ]);
    let client = client(transport);
    client.call_tool("prime", json!({})).await.unwrap();
    assert!(client.tools_list_changed());

    client.list_tools().await.unwrap();

    assert!(client.tools_list_changed());
}

fn client(transport: Arc<ScriptedTransport>) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(
        transport,
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap()
}

fn list_with_invalidation(id: u64) -> crate::McpHttpResponse {
    event_response(
        vec![
            invalidation(),
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [{"name": "one", "inputSchema": {"type": "object"}}]
                }
            }),
        ],
        Arc::new(AtomicBool::new(true)),
    )
}

fn call_with_invalidation(id: u64) -> crate::McpHttpResponse {
    event_response(
        vec![
            invalidation(),
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"content": [{"type": "text", "text": "ok"}]}
            }),
        ],
        Arc::new(AtomicBool::new(true)),
    )
}

fn invalidation() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/tools/list_changed"
    })
}
