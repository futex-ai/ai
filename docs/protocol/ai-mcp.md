# Spec: `ai-mcp` crate — MCP client + tool adapter

Status: approved by Cal (22 Jul 2026). Treat this document as the user-approved
basis for a plan under `plans/` in this repo (suggested name:
`plans/ai-mcp-crate.md`). It is self-contained; no external product context is
required.

## Purpose

Add a new workspace crate `crates/ai-mcp` that implements a client for the
Model Context Protocol (MCP, <https://modelcontextprotocol.io>) over the
streamable HTTP transport, plus an adapter that exposes a remote MCP server's
tools through the existing `ai_interface::Tool` boundary so `ai-tool-calling`
can dispatch them like any other tool group.

Downstream context (for design intent only — do not implement any of it here):
a product consumer will construct one client per configured MCP server, inject
auth headers from its own credential store via the existing
`json_http::JsonHttpAuth` hook, and handle OAuth flows, token refresh, and
storage entirely on its side. This crate must stay pure protocol + adapter.

## Goals

- Connect to a remote MCP server at a URL over streamable HTTP.
- Initialize handshake with protocol version negotiation and session tracking.
- List tools (with pagination) and call tools.
- Expose a server's tools as `ai_interface::Tool` with collision-safe,
  provider-legal tool names.
- Surface auth challenges (HTTP 401/403) as a typed error the caller can act
  on. No auth flows in this crate.
- Follow all repo conventions (dyn traits, unimock, thiserror enums, file-size
  caps, `_tests_` layout, crate README).

## Non-goals (v1)

- OAuth flows, dynamic client registration, token storage/refresh (caller-side).
- stdio transport and the legacy 2024-11-05 HTTP+SSE transport.
- MCP resources, prompts, sampling, elicitation, roots, completions.
- Server-initiated GET event stream, SSE resumability (`Last-Event-ID`),
  progress streaming, cancellation notifications.
- Automatic retries (callers own retry policy; do not wrap in
  `ai-models-core` retrying).
- JSON-RPC batching (removed in the 2025-06-18 revision — never send batches).

## Protocol target

Target revision **2025-06-18**; also accept servers that negotiate down to
**2025-03-26**. Any other server version is a typed error. If you have web
access, verify against the current spec before implementing:

- <https://modelcontextprotocol.io/specification/2025-06-18/basic/transports>
- <https://modelcontextprotocol.io/specification/2025-06-18/server/tools>
- <https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization>

Required transport behavior (streamable HTTP, single endpoint URL):

1. All client→server JSON-RPC messages are HTTP POSTs to the endpoint with
   `Content-Type: application/json` and
   `Accept: application/json, text/event-stream`.
2. For a JSON-RPC *request*, the server responds with either
   `application/json` (one JSON-RPC response object) or
   `text/event-stream` (SSE events whose `data:` payloads are JSON-RPC
   messages; the stream contains the response for the posted request and may
   also contain server notifications/requests, then closes).
3. For a client *notification* (e.g. `notifications/initialized`) or a client
   *response*, the server replies `202 Accepted` with no body.
4. `initialize` request params:
   `{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"ai-mcp","version":"<crate version>"}}`.
   The result carries `protocolVersion`, `capabilities`, `serverInfo`,
   optional `instructions`. After the result, send the
   `notifications/initialized` notification.
5. Session: if the initialize HTTP response includes an `Mcp-Session-Id`
   header, include that header on every subsequent request. A `404` on a
   request that carried a session id means the session expired
   (`Error::SessionExpired`; do not silently re-initialize in v1).
6. After negotiation, send `MCP-Protocol-Version: <negotiated>` as an HTTP
   header on every subsequent request.
7. `close()` sends HTTP DELETE with the session header; a `405` response is
   success (servers may disallow client termination).
8. `tools/list` paginates via `nextCursor`/`params.cursor` until exhausted.
9. `tools/call` params: `{"name":"<original tool name>","arguments":{...}}`.
10. Server messages that may appear inside a POST's SSE stream:
    - `notifications/tools/list_changed` → set a stale flag (see API).
    - `ping` request → POST back an empty-object result response.
    - any other server request (e.g. sampling) → POST back a JSON-RPC error
      response with code `-32601` (method not found).
    - other notifications → ignore.
11. JSON-RPC ids: monotonically increasing `u64` per client instance.

Simplification (documented tradeoff): the transport may buffer an entire SSE
response stream to completion and return all decoded JSON-RPC messages as a
batch to the client layer, which then picks the matching response and
processes the rest per rule 10. No long-lived streams in v1. Enforce
`max_response_bytes` while buffering.

## Crate layout

```
crates/ai-mcp/
  README.md            # required sections per repo README rules
  Cargo.toml
  src/
    lib.rs             # module decls + intentional exports only
    config.rs          # McpServerConfig + limits (pure data)
    error.rs           # Error/Result
    protocol/          # serde DTOs: JSON-RPC envelope, initialize, tools/list,
                       # tools/call, content blocks, capabilities, versions
    transport/         # McpHttpTransport trait, ReqwestMcpHttpTransport, SSE parser
    client.rs          # McpClient trait + StreamableHttpMcpClient
    tool_set/          # McpToolSet (impl ai_interface::Tool) + naming rules
    _tests_/           # per repo testing conventions
  tests/               # integration tests (in-process fake server)
```

Workspace wiring: add to root `Cargo.toml` `[workspace] members` and
`[workspace.dependencies]` as
`ai-mcp = { version = "0.2.0", path = "crates/ai-mcp" }` (shared workspace
version/edition/license via `.workspace = true`).

Dependencies: `ai-interface` (workspace), `json-http` (workspace; for
`JsonHttpAuth` only), `async-trait`, `serde` (derive), `serde_json`,
`thiserror`, `reqwest` (default-features = false, features
`["json", "rustls", "stream"]`), `futures-util` (stream reading). Dev:
`tokio` (macros, rt), `unimock`, `axum` (fake-server integration tests). Use
`cargo add` for external deps per repo rules. Provide a `test-support`
feature that exposes unimock mocks for the crate's traits, mirroring
`json-http`.

## Public API contract

Types below are the contract; exact module placement follows the layout above.
All traits get `#[cfg_attr(any(test, doctest, feature = "test-support"), unimock::unimock(api = ...Mock))]`.

```rust
/// Pure-data connection config.
pub struct McpServerConfig {
    /// Stable key naming this server; must match [a-z0-9_-]{1,32}.
    pub server_key: String,
    /// MCP endpoint URL (the single streamable-HTTP endpoint).
    pub url: String,
    /// Timeout for initialize/list requests. Default 30s.
    pub request_timeout: Duration,
    /// Timeout for tools/call requests. Default 120s.
    pub tool_call_timeout: Duration,
    /// Cap on any buffered HTTP/SSE response body. Default 1 MiB.
    pub max_response_bytes: usize,
    /// Optional UI activity verb applied to every exposed tool definition.
    pub activity_verb: Option<String>,
}

/// Transport-level HTTP boundary (mockable seam).
#[async_trait]
pub trait McpHttpTransport: Send + Sync {
    async fn post(&self, url: &str, headers: &BTreeMap<String, String>,
                  body: &Value, max_response_bytes: usize, timeout: Duration)
                  -> Result<McpHttpResponse>;
    async fn delete(&self, url: &str, headers: &BTreeMap<String, String>,
                    timeout: Duration) -> Result<McpHttpResponse>;
}
pub type DynMcpHttpTransport = Arc<dyn McpHttpTransport>;

/// HTTP outcome: status, response headers, and decoded payload.
pub struct McpHttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,   // lowercase header names
    pub payload: McpHttpPayload,
}
pub enum McpHttpPayload {
    None,                       // e.g. 202 Accepted
    Json(Value),                // application/json body
    EventStream(Vec<Value>),    // all JSON-RPC messages from a buffered SSE body
}

/// Protocol client boundary.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Idempotent handshake; returns negotiated version + server info.
    async fn ensure_initialized(&self) -> Result<McpServerHandshake>;
    /// Full tool list (auto-initializes, auto-paginates, clears the stale flag).
    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>>;
    /// Calls one tool by its ORIGINAL (unprefixed) name.
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolCallOutcome>;
    /// True when a tools/list_changed notification was seen since the last list_tools.
    fn tools_list_changed(&self) -> bool;
    /// Terminates the session (HTTP DELETE; 405 tolerated).
    async fn close(&self) -> Result<()>;
}
pub type DynMcpClient = Arc<dyn McpClient>;
```

`StreamableHttpMcpClient` is the single production impl:
`new(transport: DynMcpHttpTransport, auth: DynJsonHttpAuth, config: McpServerConfig)`.
Auth is applied by asking the hook to mutate the outgoing header map before
every request (same pattern as `ai-models-anthropic`, which holds a
`DynJsonHttpClient` + `DynJsonHttpAuth` pair). Callers pass
`StaticHeaderAuth::default()` for unauthenticated servers. Internal session
state (`Mcp-Session-Id`, negotiated version, id counter, stale flag) lives
behind interior mutability; the type must be `Send + Sync` and safe for
concurrent `call_tool`s.

```rust
/// One server tool as discovered from tools/list.
pub struct McpToolDescriptor {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
}

/// Result of tools/call.
pub struct McpToolCallOutcome {
    pub content: Vec<McpContentBlock>,
    pub structured_content: Option<Value>,
    pub is_error: bool,
}

/// Wire content blocks (serde tag = "type", camelCase fields per MCP schema);
/// include an untagged fallback variant preserving unknown block types as raw JSON.
pub enum McpContentBlock { Text {..}, Image {..}, Audio {..}, ResourceLink {..}, EmbeddedResource {..}, Unknown(Value) }
```

### `McpToolSet` (the `ai_interface::Tool` adapter)

- Constructor `McpToolSet::new(client: DynMcpClient, config_view, descriptors: Vec<McpToolDescriptor>) -> Result<Self>`
  precomputes prefixed `ToolDefinition`s and a `BTreeMap<String, String>`
  prefixed→original dispatch map. Convenience
  `async fn load(client, ...) -> Result<Self>` does `list_tools` then `new`.
  Hosts decide refresh cadence by re-`load`ing (e.g. per turn with their own
  cache, or when `tools_list_changed()` is true).
- `definitions()` returns the snapshot. `ToolDefinition` mapping: `name` =
  prefixed name; `description` = descriptor description, else title, else
  original name; `input_schema` = pass-through `inputSchema`; `activity_verb`
  from config.
- `group_for_tool` returns `Some("mcp")` for owned names.
- **Naming rules**: prefixed name is `mcp__{server_key}__{tool}` where `tool`
  is the original name sanitized by replacing every char outside
  `[a-zA-Z0-9_-]` with `_`. Total length must be ≤ 64 (provider tool-name
  limits): truncate the sanitized tool part to fit. On collision after
  sanitize/truncate, append `_2`, `_3`, … (re-truncating so the result stays
  ≤ 64). Reject invalid `server_key` with `Error::InvalidServerKey` at
  construction.
- **`call()` semantics** (this ordering is load-bearing):
  1. Unknown prefixed name → `Err(ToolError::UnknownTool)`.
  2. Client/protocol/transport failure → `Err(ToolError::execution(name, err))`.
  3. `is_error == true` → **`Ok`**, not `Err`: return
     `json!({"is_error": true, "content": <mapped content>})` so the model
     sees the tool's error text and can react (MCP semantics distinguish
     protocol errors from tool-execution errors).
  4. `is_error == false` and `structured_content` present → return it directly.
  5. Else if content is exactly one `Text` block → `Value::String(text)`.
  6. Else → `Value::Array` of the wire-JSON content blocks.
  - If the serialized result would exceed `max_response_bytes`, return
    `json!({"truncated": true, "content": "<utf-8-safe prefix>"})`.

## Error contract

`ai_mcp::{Error, Result}` via `thiserror`, message prefix `[ai_mcp/<mod>]`,
no catch-all variants, no `#[error(transparent)]`. Required typed variants
(callers branch on these):

| Variant | Trigger |
|---|---|
| `Unauthorized { status: u16, www_authenticate: Option<String>, resource_metadata_url: Option<String> }` | HTTP 401/403. Parse `resource_metadata="..."` out of the `WWW-Authenticate` header when present (RFC 9728 pointer); keep the raw header too. This is the hook downstream auth flows key off. |
| `SessionExpired` | 404 on a request that carried `Mcp-Session-Id` |
| `UnsupportedProtocolVersion { requested: String, server: String }` | negotiation failure |
| `JsonRpc { method: String, code: i64, message: String, data: Option<Value> }` | JSON-RPC error response |
| `MissingResponse { method: String }` | SSE stream ended without the matching response id |
| `HttpStatus { status: u16, body: Value }` | other non-success HTTP statuses |
| `ResponseTooLarge { limit_bytes: usize }` | buffering exceeded the cap |
| `DeserializeResponse { method: String, source: serde_json::Error }` | malformed payloads |
| `Transport { message: String }` | reqwest/IO failures (mirrors `json_http::Error::Transport` precedent) |
| `Auth { message: String }` | the injected `JsonHttpAuth` hook failed |
| `InvalidServerKey { server_key: String }` | config validation |

## Testing requirements

Unit tests (unimock the transport / client per repo `_tests_` conventions):

- SSE parser: multi-event bodies, multi-line `data:`, ignored `event:`/`id:`
  fields, size-cap enforcement.
- Handshake: version negotiation success + `UnsupportedProtocolVersion`,
  `Mcp-Session-Id` capture and replay, `MCP-Protocol-Version` header on
  follow-up requests, `notifications/initialized` gets 202.
- `tools/list` pagination across cursors; stale-flag set by
  `list_changed` inside a call's SSE stream and cleared by `list_tools`.
- `tools/call` mapping: structured content, single-text collapse, multi-block
  array, `is_error` → `Ok` envelope, truncation envelope.
- Server-request handling: `ping` gets an empty result reply; unsupported
  server request gets `-32601`.
- 401 with and without `resource_metadata` in `WWW-Authenticate`;
  session-expiry 404; `close()` tolerating 405.
- Naming: sanitization, 64-char truncation, collision suffixing, dispatch
  strip-prefix round-trip, `InvalidServerKey`.

Integration tests (crate-root `tests/`, in-process axum server, real
`ReqwestMcpHttpTransport`): one server serving JSON responses and one serving
SSE responses through the full initialize → list → call flow, plus a 401
challenge case. Optionally add an `#[ignore]`d live smoke test that reads a
real server URL from an env var (e.g. `AI_MCP_SMOKE_URL`) for manual runs.

## Conventions checklist (binding)

- Dyn-trait seams for all non-pure behavior; `Arc<dyn Trait + Send + Sync>`.
- No `unwrap`/`expect`/`panic!` outside tests; no `map_err` in production
  paths (use `?` and typed helpers).
- 300-line file cap; thin `lib.rs`; module-level doc comments; doc comments on
  all public items; `#![warn(unreachable_pub)]`.
- `cargo fmt --all -- --check`, clippy, full test pass, `cargo xtask check`
  before completion; crate README with the six required sections; add the
  plan to `plans/README.md`.

## Suggested milestones

1. **M1 — protocol client**: crate scaffold + workspace wiring, protocol DTOs,
   SSE parser, `ReqwestMcpHttpTransport`, `StreamableHttpMcpClient`
   (initialize/list/call/close, session, version header, typed 401), unit
   tests, crate README. Green build + tests; commit and push (Conventional
   Commits, lowercase); run `cargo xtask review` and report findings without
   auto-fixing.
2. **M2 — tool adapter**: `McpToolSet` + naming rules + result mapping +
   `Tool` impl + unit tests. Same check/commit/push/review steps.
3. **M3 — integration hardening**: axum fake-server integration tests (JSON +
   SSE + 401), optional ignored live smoke test, README/docs polish, tick the
   plan. Same check/commit/push/review steps.

## Acceptance criteria

- `McpToolSet` for a live server (e.g. one exposing a few tools) can be
  registered in `ai-tool-calling` and its tools dispatch end-to-end.
- An unauthenticated server works with `StaticHeaderAuth::default()`; a
  Bearer-protected server works with `StaticHeaderAuth::bearer_token(...)`;
  a 401 without credentials yields `Error::Unauthorized` carrying the
  `resource_metadata` URL when the server provides one.
- No auth flows, storage, or product policy anywhere in the crate.
- All workspace checks pass (`cargo xtask check`), 100% test pass rate.

## Appendix: how the downstream consumer will wire it (context only)

Per configured server: build `ReqwestMcpHttpTransport`, an implementation of
`json_http::JsonHttpAuth` backed by its credential store (refreshing tokens
before expiry inside the hook), and `StreamableHttpMcpClient` with a
`McpServerConfig`. Per agent turn it `McpToolSet::load`s (with its own
caching) and appends the set to the turn's tools. On `Error::Unauthorized` it
flips the connection into a needs-authentication state and runs its OAuth
flow (discovery via the `resource_metadata` URL, dynamic client registration,
browser redirect), then retries. None of that is this crate's concern.
