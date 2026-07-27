//! SSE side-message ordering and invalidation tests.

use std::sync::Arc;

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpClient, McpServerConfig, StreamableHttpMcpClient};

use super::{
    lifecycle_tests::initialized_response,
    support::{ScriptedTransport, empty_response, event_response},
};

#[tokio::test]
async fn replies_to_server_requests_before_polling_again() {
    let gate = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let transport = ScriptedTransport::new_with_gate(
        vec![
            initialized_response(),
            empty_response(202),
            event_response(
                vec![
                    json!({"jsonrpc":"2.0","id":"server-1","method":"ping"}),
                    json!({"jsonrpc":"2.0","method":"notifications/tools/list_changed"}),
                    json!({
                        "jsonrpc":"2.0","id":2,
                        "result":{"content":[{"type":"text","text":"ok"}]}
                    }),
                ],
                gate.clone(),
            ),
            empty_response(202),
        ],
        gate,
    );
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();
    let outcome = client.call_tool("run", json!({})).await.unwrap();

    assert!(!outcome.is_error);
    assert!(client.tools_list_changed());
    let reply = &transport.posts()[3].body;
    assert_eq!(reply["id"], "server-1");
    assert_eq!(reply["result"], json!({}));
}

#[tokio::test]
async fn echoes_numeric_ids_in_method_not_found_replies() {
    let gate = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let transport = ScriptedTransport::new_with_gate(
        vec![
            initialized_response(),
            empty_response(202),
            event_response(
                vec![
                    json!({
                        "jsonrpc":"2.0",
                        "id":9,
                        "method":"sampling/createMessage"
                    }),
                    json!({
                        "jsonrpc":"2.0","id":2,
                        "result":{"content":[{"type":"text","text":"ok"}]}
                    }),
                ],
                gate.clone(),
            ),
            empty_response(202),
        ],
        gate,
    );
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    client.call_tool("run", json!({})).await.unwrap();

    let reply = &transport.posts()[3].body;
    assert_eq!(reply["id"], 9);
    assert_eq!(reply["error"]["code"], -32601);
}

#[tokio::test]
async fn surfaces_null_id_errors_from_the_scoped_sse_stream() {
    let client = client_with_events(vec![
        json!({"jsonrpc":"2.0","method":"notifications/tools/list_changed"}),
        json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {"code": -32700, "message": "Parse error"}
        }),
    ]);

    let error = client.call_tool("run", json!({})).await.unwrap_err();

    assert!(matches!(
        error,
        Error::JsonRpc {
            method,
            code: -32700,
            message,
            data: None,
        } if method == "tools/call" && message == "Parse error"
    ));
    assert!(client.tools_list_changed());
}

#[tokio::test]
async fn ignores_unrelated_non_null_errors_before_the_matching_sse_response() {
    let client = client_with_events(vec![
        json!({
            "jsonrpc": "2.0",
            "id": 999,
            "error": {"code": -32603, "message": "Unrelated"}
        }),
        json!({
            "jsonrpc":"2.0",
            "id":2,
            "result":{"content":[{"type":"text","text":"ok"}]}
        }),
    ]);

    let outcome = client.call_tool("run", json!({})).await.unwrap();

    assert!(!outcome.is_error);
}

#[tokio::test]
async fn rejects_versionless_sse_side_message_without_replying() {
    let gate = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let transport = ScriptedTransport::new_with_gate(
        vec![
            initialized_response(),
            empty_response(202),
            event_response(
                vec![
                    json!({"id":"server-1","method":"ping"}),
                    json!({
                        "jsonrpc":"2.0",
                        "id":2,
                        "result":{"content":[{"type":"text","text":"ok"}]}
                    }),
                ],
                gate.clone(),
            ),
        ],
        gate,
    );
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let error = client.call_tool("run", json!({})).await.unwrap_err();

    assert!(matches!(
        error,
        Error::DeserializeResponse { method, .. } if method == "tools/call"
    ));
    assert_eq!(transport.posts().len(), 3);
}

#[tokio::test]
async fn rejects_null_id_tool_list_notification_without_side_effects() {
    let gate = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let transport = ScriptedTransport::new_with_gate(
        vec![
            initialized_response(),
            empty_response(202),
            event_response(
                vec![
                    json!({
                        "jsonrpc":"2.0",
                        "id":null,
                        "method":"notifications/tools/list_changed"
                    }),
                    json!({
                        "jsonrpc":"2.0",
                        "id":2,
                        "result":{"content":[{"type":"text","text":"ok"}]}
                    }),
                ],
                gate.clone(),
            ),
        ],
        gate,
    );
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let error = client.call_tool("run", json!({})).await.unwrap_err();

    assert!(matches!(
        error,
        Error::DeserializeResponse { method, .. } if method == "tools/call"
    ));
    assert!(!client.tools_list_changed());
    assert_eq!(transport.posts().len(), 3);
}

fn client_with_events(events: Vec<serde_json::Value>) -> StreamableHttpMcpClient {
    let gate = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let transport = ScriptedTransport::new_with_gate(
        vec![
            initialized_response(),
            empty_response(202),
            event_response(events, gate.clone()),
        ],
        gate,
    );
    StreamableHttpMcpClient::new(
        transport,
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap()
}
