//! Shared in-process HTTP server support for MCP integration tests.

use std::sync::Arc;

use ai_mcp::{ReqwestMcpHttpTransport, StreamableHttpMcpClient};
use axum::{Router, http::HeaderMap};
use json_http::JsonHttpAuth;
use serde_json::Value;
use tokio::{net::TcpListener, task::JoinHandle};

#[derive(Clone)]
/// One HTTP request observed by a fake MCP server.
pub(crate) struct RecordedRequest {
    pub(crate) headers: HeaderMap,
    pub(crate) body: Value,
}

/// Running in-process HTTP server whose task is aborted on drop.
pub(crate) struct TestServer {
    pub(crate) endpoint: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Starts an Axum router on an ephemeral loopback port.
pub(crate) async fn spawn(router: Router) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    TestServer {
        endpoint: format!("http://{address}/mcp"),
        task,
    }
}

/// Builds the production MCP client against an in-process endpoint.
pub(crate) fn client(endpoint: &str, auth: Arc<dyn JsonHttpAuth>) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(
        Arc::new(ReqwestMcpHttpTransport::new()),
        auth,
        ai_mcp::McpServerConfig::new("integration", endpoint),
    )
    .unwrap()
}

/// Returns a UTF-8 request-header value.
pub(crate) fn header<'a>(request: &'a RecordedRequest, name: &str) -> &'a str {
    request.headers.get(name).unwrap().to_str().unwrap()
}
