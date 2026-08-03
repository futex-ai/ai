//! Deterministic transport support for freshness races.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::Notify;

use crate::{McpHttpResponse, McpHttpTransport, Result};

use super::super::support::{empty_response, event_response, json_response};

#[derive(Clone, Copy)]
pub(super) enum CallBehavior {
    ListChanged,
    SessionExpired,
}

pub(super) struct TwoPageRaceTransport {
    call_behavior: CallBehavior,
    final_page_entered: Notify,
    final_page_release: Notify,
    initial_list_requests: AtomicUsize,
}

impl TwoPageRaceTransport {
    pub(super) fn new(call_behavior: CallBehavior) -> Self {
        Self {
            call_behavior,
            final_page_entered: Notify::new(),
            final_page_release: Notify::new(),
            initial_list_requests: AtomicUsize::new(0),
        }
    }

    pub(super) async fn wait_until_final_page(&self) {
        self.final_page_entered.notified().await;
    }

    pub(super) fn release_final_page(&self) {
        self.final_page_release.notify_one();
    }

    async fn list_response(&self, body: &Value, id: Value) -> McpHttpResponse {
        match body.pointer("/params/cursor").and_then(Value::as_str) {
            Some("older-next") => {
                self.final_page_entered.notify_one();
                self.final_page_release.notified().await;
                tool_page(id, "older-final", None)
            }
            Some(cursor) => panic!("unexpected tools/list cursor {cursor}"),
            None => match self.initial_list_requests.fetch_add(1, Ordering::SeqCst) {
                0 => tool_page(id, "older-first", Some("older-next")),
                1 => tool_page(id, "newer", None),
                request => panic!("unexpected initial tools/list request {request}"),
            },
        }
    }
}

impl Default for TwoPageRaceTransport {
    fn default() -> Self {
        Self::new(CallBehavior::ListChanged)
    }
}

#[async_trait]
impl McpHttpTransport for TwoPageRaceTransport {
    async fn post(
        &self,
        _url: &str,
        _headers: &BTreeMap<String, String>,
        body: &Value,
        _max_response_bytes: usize,
        _timeout: Duration,
    ) -> Result<McpHttpResponse> {
        let id = body.get("id").cloned().unwrap_or(Value::Null);
        match body.get("method").and_then(Value::as_str) {
            Some("initialize") => Ok(initialize_response(id)),
            Some("notifications/initialized") => Ok(empty_response(202)),
            Some("tools/list") => Ok(self.list_response(body, id).await),
            Some("tools/call") => Ok(match self.call_behavior {
                CallBehavior::ListChanged => call_with_invalidation(id),
                CallBehavior::SessionExpired => session_expired_response(),
            }),
            method => panic!("unexpected method {method:?}"),
        }
    }

    async fn delete(
        &self,
        _url: &str,
        _headers: &BTreeMap<String, String>,
        _max_response_bytes: usize,
        _timeout: Duration,
    ) -> Result<McpHttpResponse> {
        panic!("unexpected DELETE")
    }
}

fn initialize_response(id: Value) -> McpHttpResponse {
    json_response(
        200,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "serverInfo": {"name": "demo", "version": "1"}
            }
        }),
        BTreeMap::from([("mcp-session-id".to_owned(), vec!["session-1".to_owned()])]),
    )
}

fn tool_page(id: Value, name: &str, next_cursor: Option<&str>) -> McpHttpResponse {
    let mut result = json!({
        "tools": [{"name": name, "inputSchema": {"type": "object"}}]
    });
    if let Some(next_cursor) = next_cursor {
        result["nextCursor"] = json!(next_cursor);
    }
    json_response(
        200,
        json!({"jsonrpc": "2.0", "id": id, "result": result}),
        BTreeMap::new(),
    )
}

fn call_with_invalidation(id: Value) -> McpHttpResponse {
    event_response(
        vec![
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            }),
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"content": [{"type": "text", "text": "ok"}]}
            }),
        ],
        Arc::new(AtomicBool::new(true)),
    )
}

fn session_expired_response() -> McpHttpResponse {
    json_response(404, json!({"error": "expired"}), BTreeMap::new())
}
