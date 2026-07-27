//! Expired-session invalidation and recovery tests.

use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicBool},
};

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpClient, McpServerConfig, StreamableHttpMcpClient, client::RequestContext};

use super::support::{ScriptedTransport, empty_response, event_response, json_response};

#[tokio::test]
async fn host_retry_reinitializes_after_session_expiry() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(1, "session-1", "first"),
        empty_response(202),
        json_response(404, json!({"error":"gone"}), BTreeMap::new()),
        initialized_response(3, "session-2", "replacement"),
        empty_response(202),
        json_response(
            200,
            json!({"jsonrpc":"2.0","id":4,"result":{"tools":[]}}),
            BTreeMap::new(),
        ),
    ]);
    let client = client(transport.clone());

    client.ensure_initialized().await.unwrap();
    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
    assert!(client.tools_list_changed());
    assert!(client.list_tools().await.unwrap().is_empty());
    let posts = transport.posts();
    assert_eq!(
        posts
            .iter()
            .filter(|post| post.body["method"] == "initialize")
            .count(),
        2
    );
    assert!(!posts[3].headers.contains_key("Mcp-Session-Id"));
    assert_eq!(
        posts[5].headers.get("Mcp-Session-Id").map(String::as_str),
        Some("session-2")
    );
}

#[tokio::test]
async fn stale_404_does_not_erase_replacement_session() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(1, "session-2", "replacement"),
        empty_response(202),
    ]);
    let client = client(transport.clone());
    client.ensure_initialized().await.unwrap();

    let error = client
        .response_result(
            "tools/call",
            2,
            json_response(404, json!({"error":"gone"}), BTreeMap::new()),
            &RequestContext {
                session_id: Some("session-1".to_owned()),
                protocol_version: Some("2025-06-18".to_owned()),
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
    assert!(!client.tools_list_changed());
    assert_eq!(
        client.ensure_initialized().await.unwrap().server_info.name,
        "replacement"
    );
    assert_eq!(transport.posts().len(), 2);
    assert_eq!(
        client.state.lock().await.session_id.as_deref(),
        Some("session-2")
    );
}

#[tokio::test]
async fn close_404_clears_expired_session_before_returning_error() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(1, "session-1", "first"),
        empty_response(202),
        json_response(404, json!({"error":"gone"}), BTreeMap::new()),
    ]);
    let client = client(transport.clone());
    client.ensure_initialized().await.unwrap();

    let error = client.close().await.unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
    assert!(client.tools_list_changed());
    assert!(client.state.lock().await.handshake.is_none());
    client.close().await.unwrap();
    assert_eq!(transport.delete_count(), 1);
}

#[tokio::test]
async fn side_response_404_invalidates_the_scoped_session() {
    let gate = Arc::new(AtomicBool::new(false));
    let transport = ScriptedTransport::new_with_gate(
        vec![
            initialized_response(1, "session-1", "first"),
            empty_response(202),
            event_response(
                vec![
                    json!({"jsonrpc":"2.0","id":"server-1","method":"ping"}),
                    json!({
                        "jsonrpc":"2.0",
                        "id":2,
                        "result":{"content":[{"type":"text","text":"unused"}]}
                    }),
                ],
                gate.clone(),
            ),
            json_response(404, json!({"error":"gone"}), BTreeMap::new()),
        ],
        gate,
    );
    let client = client(transport);

    let error = client.call_tool("run", json!({})).await.unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
    assert!(client.tools_list_changed());
    let state = client.state.lock().await;
    assert!(state.handshake.is_none());
    assert!(state.session_id.is_none());
}

#[tokio::test]
async fn initialized_notification_404_allows_a_fresh_initialization() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(1, "session-1", "first"),
        json_response(404, json!({"error":"gone"}), BTreeMap::new()),
        initialized_response(2, "session-2", "replacement"),
        empty_response(202),
    ]);
    let client = client(transport.clone());

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
    assert!(!client.tools_list_changed());
    assert_eq!(
        client.ensure_initialized().await.unwrap().server_info.name,
        "replacement"
    );
    let posts = transport.posts();
    assert!(!posts[2].headers.contains_key("Mcp-Session-Id"));
}

fn client(transport: Arc<ScriptedTransport>) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(
        transport,
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap()
}

fn initialized_response(id: u64, session_id: &str, server_name: &str) -> crate::McpHttpResponse {
    json_response(
        200,
        json!({
            "jsonrpc":"2.0",
            "id":id,
            "result":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "serverInfo":{"name":server_name,"version":"1"}
            }
        }),
        BTreeMap::from([("mcp-session-id".to_owned(), vec![session_id.to_owned()])]),
    )
}
