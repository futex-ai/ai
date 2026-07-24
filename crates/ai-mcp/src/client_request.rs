//! Authenticated request construction for the MCP client.

use std::{collections::BTreeMap, time::Duration};

use serde_json::Value;

use crate::{Error, McpHttpResponse, Result, StreamableHttpMcpClient, client::RequestContext};

impl StreamableHttpMcpClient {
    pub(crate) async fn post_message(
        &self,
        body: &Value,
        context: &RequestContext,
        timeout: Duration,
    ) -> Result<McpHttpResponse> {
        let headers = self.authenticated_headers(context).await?;
        self.transport
            .post(
                &self.config.url,
                &headers,
                body,
                self.config.max_response_bytes,
                timeout,
            )
            .await
    }

    pub(crate) async fn post_accepted(&self, body: &Value, context: &RequestContext) -> Result<()> {
        let response = self
            .post_message(body, context, self.config.request_timeout)
            .await?;
        if response.status == 202 {
            return Ok(());
        }
        Err(self.http_error(response, context.session_id.is_some()))
    }

    pub(crate) async fn authenticated_headers(
        &self,
        context: &RequestContext,
    ) -> Result<BTreeMap<String, String>> {
        let mut headers = BTreeMap::from([
            (
                "Accept".to_owned(),
                "application/json, text/event-stream".to_owned(),
            ),
            ("Content-Type".to_owned(), "application/json".to_owned()),
        ]);
        if let Some(session_id) = &context.session_id {
            headers.insert("Mcp-Session-Id".to_owned(), session_id.clone());
        }
        if let Some(protocol_version) = &context.protocol_version {
            headers.insert("MCP-Protocol-Version".to_owned(), protocol_version.clone());
        }
        if let Err(source) = self.auth.apply_headers(&mut headers).await {
            return Err(Error::auth(&source));
        }
        Ok(headers)
    }
}
