//! Initialization wire DTOs and public handshake projection.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Successful initialization details exposed to callers.
pub struct McpServerHandshake {
    /// Protocol version selected by the server.
    pub protocol_version: String,
    /// Server implementation identity.
    pub server_info: McpServerInfo,
    /// Tool-focused server capabilities.
    pub capabilities: McpServerCapabilities,
    /// Optional server-provided usage instructions.
    pub instructions: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Server implementation identity returned during initialization.
pub struct McpServerInfo {
    /// Programmatic server name.
    pub name: String,
    /// Optional human-readable server name.
    pub title: Option<String>,
    /// Server implementation version.
    pub version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// V1 projection of server capabilities used by this tools-only client.
pub struct McpServerCapabilities {
    /// Advertised tools support, when present.
    pub tools: Option<McpToolsCapability>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Tool-specific server capability flags.
pub struct McpToolsCapability {
    /// Whether tool-list invalidation notifications may be sent.
    pub list_changed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitializeResult {
    pub(crate) protocol_version: String,
    #[serde(default)]
    pub(crate) capabilities: ServerCapabilitiesWire,
    pub(crate) server_info: McpServerInfo,
    pub(crate) instructions: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct ServerCapabilitiesWire {
    pub(crate) tools: Option<ToolsCapabilityWire>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolsCapabilityWire {
    #[serde(default)]
    pub(crate) list_changed: bool,
}

impl From<InitializeResult> for McpServerHandshake {
    fn from(value: InitializeResult) -> Self {
        Self {
            protocol_version: value.protocol_version,
            server_info: value.server_info,
            capabilities: McpServerCapabilities {
                tools: value.capabilities.tools.map(|tools| McpToolsCapability {
                    list_changed: tools.list_changed,
                }),
            },
            instructions: value.instructions,
        }
    }
}
