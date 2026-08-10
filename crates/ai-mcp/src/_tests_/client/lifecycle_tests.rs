//! Initialization, pagination, calls, and close tests.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use crate::{
    McpClient, McpHttpPayload, McpHttpResponse, McpHttpTransport, McpServerConfig, Result,
    StreamableHttpMcpClient,
};

use super::support::{ScriptedTransport, empty_response, json_response};

#[tokio::test]
async fn initializes_once_paginates_and_replays_session_headers() {
    let transport = ScriptedTransport::new(vec![
        json_response(
            200,
            json!({
                "jsonrpc":"2.0",
                "id":1,
                "result":{
                    "protocolVersion":"2025-06-18",
                    "capabilities":{"tools":{"listChanged":true}},
                    "serverInfo":{"name":"demo","version":"1"},
                    "instructions":"hello"
                }
            }),
            BTreeMap::from([("mcp-session-id".to_owned(), vec!["session-1".to_owned()])]),
        ),
        empty_response(202),
        json_response(
            200,
            json!({
                "jsonrpc":"2.0","id":2,
                "result":{
                    "tools":[{"name":"one","inputSchema":{"type":"object"}}],
                    "nextCursor":"next"
                }
            }),
            BTreeMap::new(),
        ),
        json_response(
            200,
            json!({
                "jsonrpc":"2.0","id":3,
                "result":{
                    "tools":[{"name":"two","inputSchema":{"type":"object"}}]
                }
            }),
            BTreeMap::new(),
        ),
        empty_response(405),
    ]);
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::bearer_token("secret")),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let first = client.ensure_initialized().await.unwrap();
    let second = client.ensure_initialized().await.unwrap();
    let tools = client.list_tools().await.unwrap();
    client.close().await.unwrap();

    assert_eq!(first, second);
    assert_eq!(
        tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert_eq!(transport.delete_count(), 1);
    let posts = transport.posts();
    assert_eq!(posts.len(), 4);
    assert_eq!(posts[0].body["id"], 1);
    assert_eq!(posts[1].body["method"], "notifications/initialized");
    assert_eq!(posts[2].body["params"], json!({}));
    assert_eq!(posts[3].body["params"]["cursor"], "next");
    for post in &posts[1..] {
        assert_eq!(
            post.headers.get("Mcp-Session-Id").map(String::as_str),
            Some("session-1")
        );
        assert_eq!(
            post.headers.get("MCP-Protocol-Version").map(String::as_str),
            Some("2025-06-18")
        );
        assert_eq!(
            post.headers.get("Authorization").map(String::as_str),
            Some("Bearer secret")
        );
    }
}

#[tokio::test]
async fn calls_tools_by_original_name_and_defaults_is_error() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(),
        empty_response(202),
        json_response(
            200,
            json!({
                "jsonrpc":"2.0","id":2,
                "result":{"content":[{"type":"text","text":"done"}]}
            }),
            BTreeMap::new(),
        ),
    ]);
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let outcome = client.call_tool("lookup", json!({"id": 7})).await.unwrap();

    assert!(!outcome.is_error);
    assert_eq!(transport.posts()[2].body["params"]["name"], "lookup");
}

#[tokio::test]
async fn concurrent_initialization_shares_one_handshake() {
    let transport = ScriptedTransport::new(vec![initialized_response(), empty_response(202)]);
    let client = Arc::new(
        StreamableHttpMcpClient::new(
            transport.clone(),
            Arc::new(StaticHeaderAuth::default()),
            McpServerConfig::new("demo", "https://example.com/mcp"),
        )
        .unwrap(),
    );

    let (left, right) = tokio::join!(client.ensure_initialized(), client.ensure_initialized());

    assert_eq!(left.unwrap(), right.unwrap());
    assert_eq!(transport.posts().len(), 2);
}

#[tokio::test]
async fn concurrent_tool_calls_use_distinct_monotonic_ids() {
    let transport = Arc::new(ConcurrentTransport::default());
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let (left, right) = tokio::join!(
        client.call_tool("left", json!({})),
        client.call_tool("right", json!({}))
    );

    assert!(!left.unwrap().is_error);
    assert!(!right.unwrap().is_error);
    let mut ids = transport.tool_ids.lock().unwrap().clone();
    ids.sort_unstable();
    assert_eq!(ids, [2, 3]);
}

#[tokio::test]
async fn close_surfaces_non_tolerated_statuses() {
    let transport = ScriptedTransport::new(vec![
        json_response(
            200,
            match initialized_response().payload {
                McpHttpPayload::Json(value) => value,
                _ => unreachable!(),
            },
            BTreeMap::from([("mcp-session-id".to_owned(), vec!["s".to_owned()])]),
        ),
        empty_response(202),
        json_response(500, json!({"error":"failed"}), BTreeMap::new()),
    ]);
    let client = StreamableHttpMcpClient::new(
        transport,
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();
    client.ensure_initialized().await.unwrap();

    let error = client.close().await.unwrap_err();

    assert!(matches!(
        error,
        crate::Error::HttpStatus { status: 500, .. }
    ));
}

pub(super) fn initialized_response() -> crate::McpHttpResponse {
    json_response(
        200,
        json!({
            "jsonrpc":"2.0","id":1,
            "result":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "serverInfo":{"name":"demo","version":"1"}
            }
        }),
        BTreeMap::new(),
    )
}

#[derive(Default)]
struct ConcurrentTransport {
    tool_ids: Mutex<Vec<u64>>,
}

#[async_trait]
impl McpHttpTransport for ConcurrentTransport {
    async fn post(
        &self,
        _url: &str,
        _headers: &BTreeMap<String, String>,
        body: &Value,
        _max_response_bytes: usize,
        _timeout: Duration,
    ) -> Result<McpHttpResponse> {
        let method = body.get("method").and_then(Value::as_str);
        if method == Some("notifications/initialized") {
            return Ok(empty_response(202));
        }
        let id = body["id"].as_u64().unwrap();
        if method == Some("initialize") {
            return Ok(json_response(
                200,
                json!({
                    "jsonrpc":"2.0","id":id,
                    "result":{
                        "protocolVersion":"2025-06-18",
                        "capabilities":{},
                        "serverInfo":{"name":"demo","version":"1"}
                    }
                }),
                BTreeMap::new(),
            ));
        }
        self.tool_ids.lock().unwrap().push(id);
        Ok(json_response(
            200,
            json!({
                "jsonrpc":"2.0","id":id,
                "result":{"content":[{"type":"text","text":"ok"}]}
            }),
            BTreeMap::new(),
        ))
    }

    async fn delete(
        &self,
        _url: &str,
        _headers: &BTreeMap<String, String>,
        _max_response_bytes: usize,
        _timeout: Duration,
    ) -> Result<McpHttpResponse> {
        Ok(McpHttpResponse {
            status: 405,
            headers: BTreeMap::new(),
            payload: McpHttpPayload::None,
        })
    }
}
