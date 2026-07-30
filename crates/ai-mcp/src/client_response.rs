//! HTTP and JSON-RPC response handling for the MCP client.

use serde_json::{Value, json};

use crate::{
    Error, McpHttpPayload, McpHttpResponse, McpRequestId, Result, StreamableHttpMcpClient,
    authorization::{authorization_challenge, is_authorization_status},
    client::RequestContext,
    protocol::{JsonRpcMessageKind, classify_message, error_response, success_response},
    transport::{
        content_type::{APPLICATION_JSON, first, matches},
        session_status::is_expired_session_status,
    },
};

impl StreamableHttpMcpClient {
    pub(crate) async fn response_result(
        &self,
        method: &str,
        request_id: u64,
        response: McpHttpResponse,
        context: &RequestContext,
    ) -> Result<Value> {
        if !(200..300).contains(&response.status) {
            return Err(self.scoped_http_error(response, context).await);
        }
        let content_type = first(&response.headers).map(str::to_owned);
        let is_json = matches(&response.headers, APPLICATION_JSON);
        let expected_id = McpRequestId::Number(request_id.into());
        match response.payload {
            McpHttpPayload::Json(message) => {
                if !is_json {
                    return Err(Error::UnsupportedContentType { content_type });
                }
                self.handle_message(method, &expected_id, message, context)
                    .await?
                    .ok_or_else(|| Error::MissingResponse {
                        method: method.to_owned(),
                    })
            }
            McpHttpPayload::EventStream(mut stream) => loop {
                let Some(message) = stream.next_message().await? else {
                    return Err(Error::MissingResponse {
                        method: method.to_owned(),
                    });
                };
                if let Some(result) = self
                    .handle_message(method, &expected_id, message, context)
                    .await?
                {
                    return Ok(result);
                }
            },
            McpHttpPayload::None => Err(Error::MissingResponse {
                method: method.to_owned(),
            }),
        }
    }

    async fn handle_message(
        &self,
        method: &str,
        expected_id: &McpRequestId,
        message: Value,
        context: &RequestContext,
    ) -> Result<Option<Value>> {
        match classify_message(&message) {
            JsonRpcMessageKind::Response { id, result } if &id == expected_id => Ok(Some(result)),
            JsonRpcMessageKind::Error {
                id: Some(id),
                error,
            } if &id == expected_id => Err(Error::JsonRpc {
                method: method.to_owned(),
                code: error.code,
                message: error.message,
                data: error.data,
            }),
            JsonRpcMessageKind::Error { id: None, error } => Err(Error::JsonRpc {
                method: method.to_owned(),
                code: error.code,
                message: error.message,
                data: error.data,
            }),
            JsonRpcMessageKind::Request {
                id,
                method: server_method,
            } => {
                let response = if server_method == "ping" {
                    success_response(&id, json!({}))
                } else {
                    error_response(&id, -32601, "Method not found")
                };
                self.post_accepted(&response, context).await?;
                Ok(None)
            }
            JsonRpcMessageKind::Notification {
                method: server_method,
            } => {
                if server_method == "notifications/tools/list_changed" {
                    self.tools_freshness.invalidate();
                }
                Ok(None)
            }
            JsonRpcMessageKind::Response { .. } | JsonRpcMessageKind::Error { .. } => Ok(None),
            JsonRpcMessageKind::Invalid => Err(invalid_rpc_message(method)),
        }
    }

    pub(crate) async fn scoped_http_error(
        &self,
        response: McpHttpResponse,
        context: &RequestContext,
    ) -> Error {
        let expired_session =
            if is_expired_session_status(response.status, context.session_id.is_some()) {
                context.session_id.as_deref()
            } else {
                None
            };
        let error = self.http_error(response, context.session_id.is_some());
        if let Some(expired_session) = expired_session {
            self.invalidate_expired_session(expired_session).await;
        }
        error
    }

    async fn invalidate_expired_session(&self, expired_session: &str) {
        let mut state = self.state.lock().await;
        if state.session_id.as_deref() == Some(expired_session) {
            *state = Default::default();
            self.tools_freshness.invalidate();
        }
    }

    fn http_error(&self, response: McpHttpResponse, had_session: bool) -> Error {
        let status = response.status;
        if is_authorization_status(status) {
            let raw = response
                .headers
                .get("www-authenticate")
                .cloned()
                .unwrap_or_default();
            let challenge = authorization_challenge(status, &raw);
            return if status == 401 {
                Error::AuthorizationRequired { challenge }
            } else {
                Error::Forbidden { challenge }
            };
        }
        if is_expired_session_status(status, had_session) {
            return Error::SessionExpired;
        }
        Error::HttpStatus {
            status,
            body: payload_value(response.payload),
        }
    }
}

fn payload_value(payload: McpHttpPayload) -> Value {
    match payload {
        McpHttpPayload::Json(value) => value,
        McpHttpPayload::None | McpHttpPayload::EventStream(_) => Value::Null,
    }
}

fn invalid_rpc_message(method: &str) -> Error {
    let source = serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "invalid JSON-RPC message",
    ));
    Error::deserialize(method, source)
}
