//! Initial handshake HTTP error classification tests.

use std::{collections::BTreeMap, sync::Arc};

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpClient, McpServerConfig, StreamableHttpMcpClient};

use super::support::{ScriptedTransport, empty_response, json_response};

#[tokio::test]
async fn failed_initialize_response_session_does_not_imply_expiry() {
    let transport = ScriptedTransport::new(vec![
        json_response(
            404,
            json!({"error":"not found"}),
            BTreeMap::from([(
                "mcp-session-id".to_owned(),
                vec!["failed-session".to_owned()],
            )]),
        ),
        initialized_response(),
        empty_response(202),
    ]);
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::HttpStatus { status: 404, body } if body == json!({"error":"not found"})
    ));
    assert!(!client.tools_list_changed());
    assert_eq!(
        client.ensure_initialized().await.unwrap().server_info.name,
        "replacement"
    );
    let posts = transport.posts();
    assert!(!posts[1].headers.contains_key("Mcp-Session-Id"));
    assert_eq!(
        client.state.lock().await.session_id.as_deref(),
        Some("replacement-session")
    );
}

fn initialized_response() -> crate::McpHttpResponse {
    json_response(
        200,
        json!({
            "jsonrpc":"2.0","id":2,
            "result":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "serverInfo":{"name":"replacement","version":"1"}
            }
        }),
        BTreeMap::from([(
            "mcp-session-id".to_owned(),
            vec!["replacement-session".to_owned()],
        )]),
    )
}
