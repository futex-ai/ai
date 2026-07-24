//! HTTP and JSON-RPC response handling for the MCP client.

use serde_json::{Value, json};

use crate::{
    Error, McpHttpPayload, McpHttpResponse, McpRequestId, Result, StreamableHttpMcpClient,
    authorization::authorization_challenge,
    client::RequestContext,
    protocol::{JsonRpcMessageKind, classify_message, error_response, success_response},
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
            return Err(self.http_error(response, context.session_id.is_some()));
        }
        let expected_id = McpRequestId::Number(request_id.into());
        match response.payload {
            McpHttpPayload::Json(message) => self
                .handle_message(method, &expected_id, message, context)
                .await?
                .ok_or_else(|| Error::MissingResponse {
                    method: method.to_owned(),
                }),
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
                    self.tools_stale
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Ok(None)
            }
            JsonRpcMessageKind::Response { .. } | JsonRpcMessageKind::Error { .. } => Ok(None),
            JsonRpcMessageKind::Invalid => Err(invalid_rpc_message(method)),
        }
    }

    pub(crate) fn http_error(&self, response: McpHttpResponse, had_session: bool) -> Error {
        let status = response.status;
        if status == 401 || status == 403 {
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
        if status == 404 && had_session {
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
