//! Initialized-context invalidation race regressions.

use std::{collections::BTreeMap, sync::Arc};

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpClient, McpServerConfig, StreamableHttpMcpClient, client::RequestContext};

use super::support::{ScriptedTransport, empty_response, json_response};

#[tokio::test]
async fn operation_context_survives_invalidation_between_state_reads() {
    let transport = ScriptedTransport::new(vec![
        initialized_response(1, "session-1"),
        empty_response(202),
        json_response(404, json!({"error":"gone"}), BTreeMap::new()),
    ]);
    let client = Arc::new(client(transport.clone()));
    client.ensure_initialized().await.unwrap();
    let (calling, expiring) = {
        let _state_guard = client.state.lock().await;
        let call_client = client.clone();
        let calling = tokio::spawn(async move { call_client.call_tool("run", json!({})).await });
        tokio::task::yield_now().await;
        let expiry_client = client.clone();
        let expiring = tokio::spawn(async move {
            expiry_client
                .response_result(
                    "tools/call",
                    99,
                    json_response(404, json!({"error":"gone"}), BTreeMap::new()),
                    &RequestContext {
                        session_id: Some("session-1".to_owned()),
                        protocol_version: Some("2025-06-18".to_owned()),
                    },
                )
                .await
        });
        tokio::task::yield_now().await;
        (calling, expiring)
    };

    let call_error = calling.await.unwrap().unwrap_err();
    let expiry_error = expiring.await.unwrap().unwrap_err();

    assert!(matches!(expiry_error, Error::SessionExpired));
    assert!(
        matches!(call_error, Error::SessionExpired),
        "unexpected call error: {call_error:?}"
    );
    let posts = transport.posts();
    assert_eq!(posts.len(), 3);
    assert_eq!(posts[2].body["method"], "tools/call");
    assert_eq!(
        posts[2].headers.get("Mcp-Session-Id").map(String::as_str),
        Some("session-1")
    );
}

fn client(transport: Arc<ScriptedTransport>) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(
        transport,
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap()
}

fn initialized_response(id: u64, session_id: &str) -> crate::McpHttpResponse {
    json_response(
        200,
        json!({
            "jsonrpc":"2.0",
            "id":id,
            "result":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "serverInfo":{"name":"server","version":"1"}
            }
        }),
        BTreeMap::from([("mcp-session-id".to_owned(), vec![session_id.to_owned()])]),
    )
}
