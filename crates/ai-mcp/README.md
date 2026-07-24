# ai-mcp

`ai-mcp` is the workspace's protocol-focused Model Context Protocol client.
Depend on it when a caller needs tools from a remote MCP server using the
2025-06-18 streamable HTTP transport. Authentication policy and OAuth flows
belong to the host and the companion `ai-mcp-oauth` crate.

## Responsibilities

- Initialize streamable HTTP MCP sessions and negotiate supported versions.
- Discover and call server tools through typed protocol DTOs.
- Preserve session state, authorization challenges, and tool-list
  invalidations.
- Decode JSON or incremental SSE responses within configured size limits.

## What This Crate Does

`StreamableHttpMcpClient` sends one JSON-RPC message per HTTP request through a
trait-backed transport. It supports protocol versions `2025-06-18` and
`2025-03-26`, session and protocol headers, paginated tool discovery, tool
calls, server pings, tool-list invalidation, and session termination.

`McpToolSet` snapshots discovered tools for `ai-interface` and
`ai-tool-calling`. Names become `mcp__{server_key}__{sanitized_tool}` with
collision suffixes and a 64-character limit. Hosts own refresh cadence: build a
new snapshot when `tools_list_changed()` is true or according to product cache
policy.

HTTP authentication is injected through `json_http::JsonHttpAuth`. The crate
surfaces typed `AuthorizationRequired` and `Forbidden` errors but never opens a
browser, stores credentials, or retries authorization.

## Quick Start

```rust,no_run
use std::sync::Arc;

use ai_mcp::{
    McpClient, McpServerConfig, ReqwestMcpHttpTransport,
    StreamableHttpMcpClient,
};
use json_http::StaticHeaderAuth;

async fn list_remote_tools() -> ai_mcp::Result<Vec<String>> {
    let config = McpServerConfig::new("calendar", "https://example.com/mcp");
    let client = Arc::new(StreamableHttpMcpClient::new(
        Arc::new(ReqwestMcpHttpTransport::new()),
        Arc::new(StaticHeaderAuth::default()),
        config,
    )?);
    Ok(client
        .list_tools()
        .await?
        .into_iter()
        .map(|tool| tool.name)
        .collect())
}
```

Expose a discovered snapshot through the shared tool boundary:

```rust,no_run
use std::sync::Arc;

use ai_interface::Tool;
use ai_mcp::{DynMcpClient, McpServerConfig, McpToolSet};

async fn load_adapter(
    client: DynMcpClient,
    config: &McpServerConfig,
) -> ai_mcp::Result<Arc<dyn Tool>> {
    Ok(Arc::new(McpToolSet::load(client, config).await?))
}
```

Register the returned `Arc<dyn Tool>` in
`ai_tool_calling::ToolCallingRuntime`. Structured MCP results pass through,
single text blocks collapse to strings, multi-block results retain their MCP
wire JSON, and remote `isError` results remain successful model-visible error
envelopes. Protocol and transport failures become `ToolError::Execution`.

For a fixed Bearer credential, replace the default auth hook with:

```rust
# use std::sync::Arc;
# use json_http::StaticHeaderAuth;
let auth = Arc::new(StaticHeaderAuth::bearer_token("access-token"));
```

## Development

```sh
cargo test -p ai-mcp --all-features
cargo clippy -p ai-mcp --all-targets --all-features -- -D warnings
cargo xtask rust-file-length-lint --all
```

### Key Code

- `src/client.rs` — public client trait and synchronized runtime state
- `src/client_operations.rs` — initialization, tools, and close operations
- `src/transport/` — mockable HTTP boundary and incremental SSE transport
- `src/protocol/` — typed MCP wire DTOs and content blocks
- `src/authorization.rs` — typed Bearer challenge parsing
- `src/tool_set.rs` — immutable `ai_interface::Tool` adapter
- `src/tool_set_naming.rs` — namespacing, sanitization, and collision handling
- `src/tool_set_result.rs` — result precedence and UTF-8-safe truncation

### Related Docs

- [`../../docs/protocol/ai-mcp.md`](../../docs/protocol/ai-mcp.md)
- [`../../docs/protocol/mcp-oauth.md`](../../docs/protocol/mcp-oauth.md)
- [`../../plans/ai-mcp-crate.md`](../../plans/ai-mcp-crate.md)
- [`../../plans/README.md`](../../plans/README.md)
