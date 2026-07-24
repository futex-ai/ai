//! High-level MCP initialization, tool, and session operations.

use std::sync::atomic::Ordering;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{
    Error, McpClient, McpServerHandshake, McpToolCallOutcome, McpToolDescriptor, Result,
    StreamableHttpMcpClient,
    client::{COMPATIBLE_PROTOCOL_VERSION, ClientState, LATEST_PROTOCOL_VERSION, RequestContext},
    protocol::{InitializeResult, ListToolsResult, notification, request},
};

#[async_trait]
impl McpClient for StreamableHttpMcpClient {
    async fn ensure_initialized(&self) -> Result<McpServerHandshake> {
        if let Some(handshake) = self.state.lock().await.handshake.clone() {
            return Ok(handshake);
        }
        let _initialization = self.initialization_lock.lock().await;
        if let Some(handshake) = self.state.lock().await.handshake.clone() {
            return Ok(handshake);
        }

        let id = self.allocate_request_id()?;
        let empty_context = RequestContext {
            session_id: None,
            protocol_version: None,
        };
        let message = request(
            id,
            "initialize",
            json!({
                "protocolVersion": LATEST_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "ai-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        );
        let response = self
            .post_message(&message, &empty_context, self.config.request_timeout)
            .await?;
        let session_id = response
            .headers
            .get("mcp-session-id")
            .and_then(|values| values.first())
            .cloned();
        let provisional_context = RequestContext {
            session_id: session_id.clone(),
            protocol_version: Some(LATEST_PROTOCOL_VERSION.to_owned()),
        };
        let result = self
            .response_result("initialize", id, response, &provisional_context)
            .await?;
        let initialized: InitializeResult = decode_result("initialize", result)?;
        ensure_supported_version(&initialized.protocol_version)?;
        let handshake = McpServerHandshake::from(initialized);
        let context = RequestContext {
            session_id: session_id.clone(),
            protocol_version: Some(handshake.protocol_version.clone()),
        };
        self.post_accepted(
            &notification("notifications/initialized", json!({})),
            &context,
        )
        .await?;
        *self.state.lock().await = ClientState {
            handshake: Some(handshake.clone()),
            session_id,
        };
        Ok(handshake)
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>> {
        self.ensure_initialized().await?;
        let context = self.request_context().await?;
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let id = self.allocate_request_id()?;
            let params = cursor
                .as_ref()
                .map_or_else(|| json!({}), |cursor| json!({"cursor": cursor}));
            let response = self
                .post_message(
                    &request(id, "tools/list", params),
                    &context,
                    self.config.request_timeout,
                )
                .await?;
            let page: ListToolsResult = decode_result(
                "tools/list",
                self.response_result("tools/list", id, response, &context)
                    .await?,
            )?;
            tools.extend(page.tools);
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        self.tools_stale.store(false, Ordering::SeqCst);
        Ok(tools)
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolCallOutcome> {
        self.ensure_initialized().await?;
        let context = self.request_context().await?;
        let id = self.allocate_request_id()?;
        let response = self
            .post_message(
                &request(
                    id,
                    "tools/call",
                    json!({"name": name, "arguments": arguments}),
                ),
                &context,
                self.config.tool_call_timeout,
            )
            .await?;
        decode_result(
            "tools/call",
            self.response_result("tools/call", id, response, &context)
                .await?,
        )
    }

    fn tools_list_changed(&self) -> bool {
        self.tools_stale.load(Ordering::SeqCst)
    }

    async fn close(&self) -> Result<()> {
        let context = match self.optional_request_context().await {
            Some(context) if context.session_id.is_some() => context,
            _ => return Ok(()),
        };
        let headers = self.authenticated_headers(&context).await?;
        let response = self
            .transport
            .delete(
                &self.config.url,
                &headers,
                self.config.max_response_bytes,
                self.config.request_timeout,
            )
            .await?;
        if !(200..300).contains(&response.status) && response.status != 405 {
            return Err(self.http_error(response, true));
        }
        *self.state.lock().await = ClientState::default();
        Ok(())
    }
}

impl StreamableHttpMcpClient {
    fn allocate_request_id(&self) -> Result<u64> {
        match self
            .next_request_id
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            }) {
            Ok(id) => Ok(id),
            Err(_) => Err(Error::RequestIdExhausted),
        }
    }

    async fn request_context(&self) -> Result<RequestContext> {
        self.optional_request_context()
            .await
            .ok_or_else(|| Error::MissingResponse {
                method: "initialize".to_owned(),
            })
    }

    async fn optional_request_context(&self) -> Option<RequestContext> {
        let state = self.state.lock().await;
        state.handshake.as_ref().map(|handshake| RequestContext {
            session_id: state.session_id.clone(),
            protocol_version: Some(handshake.protocol_version.clone()),
        })
    }
}

fn ensure_supported_version(version: &str) -> Result<()> {
    if matches!(
        version,
        LATEST_PROTOCOL_VERSION | COMPATIBLE_PROTOCOL_VERSION
    ) {
        Ok(())
    } else {
        Err(Error::UnsupportedProtocolVersion {
            requested: LATEST_PROTOCOL_VERSION.to_owned(),
            server: version.to_owned(),
        })
    }
}

fn decode_result<T>(method: &str, value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    match serde_json::from_value(value) {
        Ok(decoded) => Ok(decoded),
        Err(source) => Err(Error::deserialize(method, source)),
    }
}
