//! Credential-free runtime construction smoke test.

use std::sync::Arc;

use ai_interface::{DynModel, MockModel, NoopLogger, Tool, ToolDefinition};
use ai_mcp::{
    McpClient, McpContentBlock, McpServerCapabilities, McpServerConfig, McpServerHandshake,
    McpServerInfo, McpToolCallOutcome, McpToolDescriptor, McpToolSet, ReqwestMcpHttpTransport,
    StreamableHttpMcpClient,
};
use ai_mcp_oauth::{
    CanonicalMcpResource, OAuthCredentialKey, OAuthRequestTokenProvider, OAuthUrlPolicy,
    RefreshingMcpAuth,
};
use ai_models_anthropic::{AnthropicModel, CLAUDE_SONNET_4_6};
use ai_models_google::{GEMINI_2_5_PRO, GoogleModel};
use ai_models_openai::{GPT_5_5, OpenAiAudioTranscriber, OpenAiModel};
use ai_models_xai::{GROK_4_20_REASONING, XaiModel};
use ai_tool_calling::ToolCallingRuntime;
use async_trait::async_trait;
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};
use serde_json::{Value, json};

use crate::error::{Error, Result};

pub(crate) fn run() -> Result<()> {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let _anthropic = AnthropicModel::new(client.clone(), CLAUDE_SONNET_4_6, "anthropic-key");
    let _google = GoogleModel::new(client.clone(), GEMINI_2_5_PRO, "google-key");
    let _openai = OpenAiModel::new(client.clone(), GPT_5_5, "openai-key");
    let _xai = XaiModel::new(client, GROK_4_20_REASONING, "xai-key");
    let _transcriber = OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "openai-key");

    let model: DynModel = Arc::new(MockModel::new("mock"));
    let tool: Arc<dyn Tool> = Arc::new(SmokeTool);
    let mcp_tool: Arc<dyn Tool> = Arc::new(build_mcp_tool()?);
    let _oauth_mcp_client = build_oauth_mcp_client()?;
    match ToolCallingRuntime::new(
        "Use registered tools when helpful.",
        model,
        Arc::new(NoopLogger),
        vec![tool, mcp_tool],
    ) {
        Ok(_runtime) => Ok(()),
        Err(source) => Err(Error::SmokeRuntime { source }),
    }
}

fn build_oauth_mcp_client() -> Result<StreamableHttpMcpClient> {
    let endpoint = "https://example.invalid/mcp";
    let resource = match CanonicalMcpResource::parse(endpoint, &OAuthUrlPolicy::default()) {
        Ok(resource) => resource,
        Err(source) => return Err(Error::SmokeMcpOAuth { source }),
    };
    let key = OAuthCredentialKey {
        account_id: "smoke".to_owned(),
        resource: resource.clone(),
        issuer: "https://auth.example.invalid".to_owned(),
        client_id: "smoke-public-client".to_owned(),
        redirect_uri: "https://app.example.invalid/oauth/callback".to_owned(),
    };
    let auth = match RefreshingMcpAuth::new(resource, key, Arc::new(EmptyTokenProvider)) {
        Ok(auth) => Arc::new(auth),
        Err(source) => return Err(Error::SmokeMcpOAuth { source }),
    };
    match StreamableHttpMcpClient::new(
        Arc::new(ReqwestMcpHttpTransport::new()),
        auth,
        McpServerConfig::new("oauth_smoke", endpoint),
    ) {
        Ok(client) => Ok(client),
        Err(source) => Err(Error::SmokeMcp { source }),
    }
}

fn build_mcp_tool() -> Result<McpToolSet> {
    let descriptors = vec![mcp_descriptor()];
    match McpToolSet::new(
        Arc::new(SmokeMcpClient),
        &McpServerConfig::new("smoke", "https://example.invalid/mcp"),
        descriptors,
    ) {
        Ok(tool) => Ok(tool),
        Err(source) => Err(Error::SmokeMcp { source }),
    }
}

fn mcp_descriptor() -> McpToolDescriptor {
    McpToolDescriptor {
        name: "remote_echo".to_owned(),
        title: None,
        description: Some("Return a remote payload.".to_owned()),
        input_schema: json!({"type": "object"}),
        output_schema: None,
    }
}

struct SmokeTool;

#[async_trait]
impl Tool for SmokeTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "smoke_echo".to_owned(),
            description: "Return the provided payload.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                }
            }),
            activity_verb: Some("Checking".to_owned()),
        }]
    }

    async fn call(&self, tool_name: &str, input: Value) -> ai_interface::ToolResult<Value> {
        let _ = tool_name;
        Ok(input)
    }

    fn group_for_tool(&self, tool_name: &str) -> Option<&'static str> {
        let _ = tool_name;
        Some("smoke")
    }
}

struct SmokeMcpClient;

struct EmptyTokenProvider;

#[async_trait]
impl OAuthRequestTokenProvider for EmptyTokenProvider {
    async fn token_for_request(
        &self,
        _key: &OAuthCredentialKey,
    ) -> ai_mcp_oauth::Result<Option<secrecy::SecretString>> {
        Ok(None)
    }
}

#[async_trait]
impl McpClient for SmokeMcpClient {
    async fn ensure_initialized(&self) -> ai_mcp::Result<McpServerHandshake> {
        Ok(McpServerHandshake {
            protocol_version: "2025-06-18".to_owned(),
            server_info: McpServerInfo {
                name: "smoke".to_owned(),
                title: None,
                version: "1".to_owned(),
            },
            capabilities: McpServerCapabilities::default(),
            instructions: None,
        })
    }

    async fn list_tools(&self) -> ai_mcp::Result<Vec<McpToolDescriptor>> {
        Ok(vec![mcp_descriptor()])
    }

    async fn call_tool(&self, _name: &str, arguments: Value) -> ai_mcp::Result<McpToolCallOutcome> {
        Ok(McpToolCallOutcome {
            content: vec![McpContentBlock::Text {
                text: arguments.to_string(),
                annotations: None,
                meta: None,
            }],
            structured_content: None,
            is_error: false,
        })
    }

    fn tools_list_changed(&self) -> bool {
        false
    }

    async fn close(&self) -> ai_mcp::Result<()> {
        Ok(())
    }
}
