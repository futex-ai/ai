//! JSON-RPC version validation through the client boundary.

use std::{collections::BTreeMap, sync::Arc};

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpServerConfig, StreamableHttpMcpClient, client::RequestContext};

use super::support::{ScriptedTransport, json_response};

#[tokio::test]
async fn rejects_json_response_without_json_rpc_version() {
    let response = json_response(200, json!({"id":1,"result":{"ok":true}}), BTreeMap::new());
    let client = StreamableHttpMcpClient::new(
        ScriptedTransport::new(Vec::new()),
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let error = client
        .response_result(
            "tools/call",
            1,
            response,
            &RequestContext {
                session_id: None,
                protocol_version: Some("2025-06-18".to_owned()),
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::DeserializeResponse { method, .. } if method == "tools/call"
    ));
}
