//! In-process protected-resource, authorization, and MCP server.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use axum::{
    Router,
    routing::{get, post},
};
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};

mod mcp;
mod oauth;

#[derive(Clone, Debug, Default)]
pub(crate) struct ServerBehavior {
    pub(crate) deny_authorization: bool,
    pub(crate) granted_scope: Option<String>,
    pub(crate) refresh_invalid_grant: bool,
    pub(crate) refresh_delay: Duration,
    pub(crate) reject_authorized: bool,
    pub(crate) insufficient_scope: bool,
    pub(crate) forbidden: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ServerRecords {
    pub(crate) authorization_queries: Vec<BTreeMap<String, String>>,
    pub(crate) registrations: Vec<Value>,
    pub(crate) token_forms: Vec<BTreeMap<String, String>>,
    pub(crate) revocation_forms: Vec<BTreeMap<String, String>>,
    pub(crate) mcp_requests: Vec<McpRequestRecord>,
}

#[derive(Clone, Debug)]
pub(crate) struct McpRequestRecord {
    pub(crate) http_method: &'static str,
    pub(crate) authorization: Option<String>,
    pub(crate) body: Option<Value>,
}

pub(crate) struct FakeOAuthMcpServer {
    pub(crate) base_url: String,
    pub(crate) mcp_url: String,
    state: Arc<ServerState>,
    task: JoinHandle<()>,
}

impl FakeOAuthMcpServer {
    pub(crate) async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = format!("http://{address}");
        let state = Arc::new(ServerState {
            base_url: base_url.clone(),
            behavior: Mutex::new(ServerBehavior::default()),
            records: Mutex::new(ServerRecords::default()),
        });
        let router = Router::new()
            .route(
                "/.well-known/oauth-protected-resource/mcp",
                get(oauth::protected_resource),
            )
            .route(
                "/.well-known/oauth-authorization-server",
                get(oauth::authorization_server),
            )
            .route("/register", post(oauth::register))
            .route("/authorize", get(oauth::authorize))
            .route("/token", post(oauth::token))
            .route("/revoke", post(oauth::revoke))
            .route("/mcp", post(mcp::post_request).delete(mcp::delete_request))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            mcp_url: format!("{base_url}/mcp"),
            base_url,
            state,
            task,
        }
    }

    pub(crate) async fn configure(&self, update: impl FnOnce(&mut ServerBehavior)) {
        let mut behavior = self.state.behavior.lock().await;
        update(&mut behavior);
    }

    pub(crate) async fn records(&self) -> ServerRecords {
        self.state.records.lock().await.clone()
    }
}

impl Drop for FakeOAuthMcpServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(super) struct ServerState {
    pub(super) base_url: String,
    pub(super) behavior: Mutex<ServerBehavior>,
    pub(super) records: Mutex<ServerRecords>,
}
