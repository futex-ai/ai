# Spec: `ai-mcp` crate — MCP client + tool adapter

Status: approved by Cal (22 Jul 2026), with review clarifications incorporated
23–24 Jul 2026. Treat this document as the user-approved basis for
`plans/ai-mcp-crate.md` in this repo. It is self-contained; no external product
context is required.

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
storage through the companion [`ai-mcp-oauth`](mcp-oauth.md) boundary. This
crate must stay pure protocol + adapter.

## Goals

- Connect to a remote MCP server at a URL over streamable HTTP.
- Initialize handshake with protocol version negotiation and session tracking.
- List tools (with pagination) and call tools.
- Expose a server's tools as `ai_interface::Tool` with collision-safe,
  provider-legal tool names.
- Surface typed HTTP 401 and 403 challenges, including OAuth discovery and
  scope hints, for a caller to act on. No auth flows in this crate.
- Follow all repo conventions (dyn traits, unimock, thiserror enums, file-size
  caps, `_tests_` layout, crate README).

## Non-goals (v1)

- OAuth flows, dynamic client registration, browser interaction, and token
  storage/refresh (owned by the companion crate and its host).
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
- <https://modelcontextprotocol.io/specification/2025-06-18/schema>
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
11. Client-generated JSON-RPC request ids are monotonically increasing `u64`
    values per client instance. Incoming server-request ids may be JSON strings
    or numbers; deserialize them into an untagged typed id enum and echo the
    exact value in the response without coercion.

The transport may buffer only enough data to decode the next complete SSE
event. It must return a live, pull-based event stream as soon as the HTTP status
and headers are available; it must not wait for the response body to reach EOF.
The client consumes JSON-RPC messages in arrival order and handles each rule 10
message before requesting the next event. In particular, it POSTs a response
to a server request while the original SSE response remains open, then resumes
that stream. The call completes when the matching JSON-RPC response arrives;
EOF before that response is `Error::MissingResponse`.

No long-lived GET streams are added in v1. A POST response stream remains scoped
to its originating request. Enforce `max_response_bytes` against cumulative raw
response bytes as they are read, including SSE framing, and fail immediately
when the limit is crossed.

The approved `SessionExpired` behavior deliberately leaves recovery to the
host. MCP 2025-06-18 transport text instead says a client receiving a
session-bound `404` must start a new session. Implementations of this contract
must not silently change behavior; revising it requires explicit protocol
approval because this crate's no-automatic-retry boundary is intentional.

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
    transport/         # HTTP/event-stream traits, reqwest transport, SSE parser
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
`["json", "rustls", "stream"]`), `futures-util` (stream reading), and optional
`unimock`. Define `test-support = ["dep:unimock"]`; also keep `unimock` as a
dev-dependency for ordinary tests. Other dev-dependencies are `tokio` (macros,
rt) and `axum` (fake-server integration tests). Use `cargo add` for external
deps per repo rules. The feature exposes unimock mocks for the crate's traits,
mirroring `json-http`.

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
    /// Cap on bytes read from any HTTP response body. Default 1 MiB.
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
                    max_response_bytes: usize, timeout: Duration)
                    -> Result<McpHttpResponse>;
}
pub type DynMcpHttpTransport = Arc<dyn McpHttpTransport>;

/// Pull-based decoded SSE response scoped to one POST request.
#[async_trait]
pub trait McpEventStream: Send {
    /// Returns the next JSON-RPC message as soon as its SSE event is complete.
    ///
    /// Returns `None` only when the response body reaches EOF.
    async fn next_message(&mut self) -> Result<Option<Value>>;
}
pub type DynMcpEventStream = Box<dyn McpEventStream>;

/// HTTP outcome: status, response headers, and decoded payload.
pub struct McpHttpResponse {
    pub status: u16,
    /// Lowercase names mapped to every field value in wire order.
    pub headers: BTreeMap<String, Vec<String>>,
    pub payload: McpHttpPayload,
}
pub enum McpHttpPayload {
    None,                               // e.g. 202 Accepted
    Json(Value),                        // capped application/json body
    EventStream(DynMcpEventStream),     // live, capped SSE response body
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

/// Successful initialization details exposed to callers.
pub struct McpServerHandshake {
    /// Protocol version selected by the server and accepted by the client.
    pub protocol_version: String,
    /// Server implementation identity from `serverInfo`.
    pub server_info: McpServerInfo,
    /// Tool-focused projection of the server's advertised capabilities.
    pub capabilities: McpServerCapabilities,
    /// Optional server-provided usage instructions.
    pub instructions: Option<String>,
}

/// Server implementation identity returned during initialization.
pub struct McpServerInfo {
    /// Programmatic server implementation name.
    pub name: String,
    /// Optional human-readable server name.
    pub title: Option<String>,
    /// Server implementation version.
    pub version: String,
}

/// V1 projection of server capabilities used by this tools-only client.
pub struct McpServerCapabilities {
    /// Advertised tools support, or `None` when the server omitted it.
    pub tools: Option<McpToolsCapability>,
}

/// Tool-specific server capability flags.
pub struct McpToolsCapability {
    /// Whether the server may emit `notifications/tools/list_changed`.
    pub list_changed: bool,
}
```

The initialization DTOs use MCP camel-case wire names.
`McpServerCapabilities` intentionally projects only the `tools` capability;
logging, prompts, resources, completions, and experimental capabilities are
ignored because they are v1 non-goals. A missing wire `listChanged` field maps
to `false`.

`StreamableHttpMcpClient` is the single production impl:
`new(transport: DynMcpHttpTransport, auth: DynJsonHttpAuth, config: McpServerConfig)`.
The client builds the protocol/session header map, asks the auth hook to mutate
that map immediately before every POST or DELETE, and passes the completed
headers across the transport seam. `McpHttpTransport` never owns or applies
authentication. This follows the `json-http` request-builder pattern while
keeping the MCP transport independently mockable. Callers pass
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

/// Optional content annotations.
pub struct McpAnnotations {
    pub audience: Option<Vec<McpRole>>,
    pub priority: Option<serde_json::Number>,
    pub last_modified: Option<String>,
}

/// Intended audience for annotated content.
pub enum McpRole {
    User,
    Assistant,
}

/// Wire content blocks. Every known variant also carries optional
/// `annotations` and `_meta` fields.
pub enum McpContentBlock {
    /// Wire type `text`.
    Text {
        text: String,
        annotations: Option<McpAnnotations>,
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Wire type `image`; `data` is base64 encoded.
    Image {
        data: String,
        mime_type: String,
        annotations: Option<McpAnnotations>,
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Wire type `audio`; `data` is base64 encoded.
    Audio {
        data: String,
        mime_type: String,
        annotations: Option<McpAnnotations>,
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Wire type `resource_link`.
    ResourceLink {
        name: String,
        title: Option<String>,
        uri: String,
        description: Option<String>,
        mime_type: Option<String>,
        annotations: Option<McpAnnotations>,
        size: Option<serde_json::Number>,
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Wire type `resource`.
    EmbeddedResource {
        resource: McpResourceContents,
        annotations: Option<McpAnnotations>,
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Any unrecognized `type`, preserved exactly for forward compatibility.
    Unknown(Value),
}

/// Contents nested inside an embedded resource.
pub enum McpResourceContents {
    Text {
        uri: String,
        mime_type: Option<String>,
        meta: Option<BTreeMap<String, Value>>,
        text: String,
    },
    Blob {
        uri: String,
        mime_type: Option<String>,
        meta: Option<BTreeMap<String, Value>>,
        blob: String,
    },
}
```

Wire serde names are `mimeType`, `lastModified`, and `_meta`; optional fields
default to `None`. `McpRole` uses lowercase `user` and `assistant`.
`McpResourceContents` is untagged and selected by the exclusive presence of
`text` or `blob`; both or neither is malformed. Content-block deserialization
uses `type` to select the five known variants (`text`, `image`, `audio`,
`resource_link`, `resource`). An unrecognized type becomes `Unknown` with the
original object intact, while a malformed known type returns
`Error::DeserializeResponse`. Serialization emits the same wire shapes and
emits an `Unknown` value unchanged. `structured_content` maps to
`structuredContent`; omitted `isError` maps to `false`.

### `McpToolSet` (the `ai_interface::Tool` adapter)

- Constructor `McpToolSet::new(client: DynMcpClient, config: &McpServerConfig, descriptors: Vec<McpToolDescriptor>) -> Result<Self>`
  precomputes prefixed `ToolDefinition`s and a `BTreeMap<String, String>`
  prefixed→original dispatch map and copies the server key, activity verb, and
  response-size limit it needs from `config`. Convenience
  `async fn load(client: DynMcpClient, config: &McpServerConfig) -> Result<Self>`
  does `list_tools` then `new`. Hosts decide refresh cadence by re-`load`ing
  (e.g. per turn with their own cache, or when `tools_list_changed()` is true).
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

```rust
/// Actionable Bearer challenge details from one 401 or 403 response.
pub struct McpAuthorizationChallenge {
    pub failure: McpAuthorizationFailure,
    pub resource_metadata_url: Option<String>,
    pub scopes: Vec<String>,
    pub error_description: Option<String>,
    pub raw_www_authenticate: Vec<String>,
}

/// Typed authorization outcome inferred from status and Bearer parameters.
pub enum McpAuthorizationFailure {
    AuthorizationRequired,
    InvalidRequest,
    InvalidToken,
    InsufficientScope,
    Forbidden,
}
```

Parse all `WWW-Authenticate` field values with an RFC-aware challenge parser;
commas inside quoted values are not separators. Preserve every raw field value.
Map the standard Bearer `error` values to the typed failure variants, split
`scope` on ASCII spaces, deduplicate scopes in first-seen order, and accept a
`resource_metadata` value only when all syntactically decoded occurrences
agree. Missing,
malformed, or conflicting optional parameters remain absent while the 401/403
itself stays actionable. A 401 without a recognized error is
`AuthorizationRequired`; a 403 without `insufficient_scope` is `Forbidden`.

| Variant | Trigger |
|---|---|
| `AuthorizationRequired { challenge: McpAuthorizationChallenge }` | HTTP 401, including missing or invalid access tokens |
| `Forbidden { challenge: McpAuthorizationChallenge }` | HTTP 403, including insufficient scope or denied permission |
| `SessionExpired` | 404 on a request that carried `Mcp-Session-Id` |
| `UnsupportedProtocolVersion { requested: String, server: String }` | negotiation failure |
| `JsonRpc { method: String, code: i64, message: String, data: Option<Value> }` | JSON-RPC error response |
| `MissingResponse { method: String }` | SSE stream ended without the matching response id |
| `HttpStatus { status: u16, body: Value }` | other non-success HTTP statuses |
| `ResponseTooLarge { limit_bytes: usize }` | cumulative response bytes exceeded the cap |
| `DeserializeResponse { method: String, source: serde_json::Error }` | malformed payloads |
| `Transport { message: String }` | reqwest/IO failures (mirrors `json_http::Error::Transport` precedent) |
| `Auth { message: String }` | the injected `JsonHttpAuth` hook failed |
| `InvalidServerKey { server_key: String }` | config validation |

## Testing requirements

Unit tests (unimock the transport / client per repo `_tests_` conventions):

- SSE parser: chunk-split and multi-event bodies, multi-line `data:`, ignored
  `event:`/`id:` fields, yielding completed events before EOF, and cumulative
  size-cap enforcement.
- Handshake: version negotiation success + `UnsupportedProtocolVersion`,
  public handshake projection for server identity, tools capability, and
  instructions, omitted `listChanged` defaulting to false, `Mcp-Session-Id`
  capture and replay, `MCP-Protocol-Version` header on follow-up requests, and
  `notifications/initialized` getting 202.
- `tools/list` pagination across cursors; stale-flag set by
  `list_changed` inside a call's SSE stream and cleared by `list_tools`.
- `tools/call` mapping: structured content, single-text collapse, multi-block
  array, `is_error` → `Ok` envelope, truncation envelope.
- Server-request handling: `ping` gets an empty result reply and unsupported
  server requests get `-32601`, with each response POST completed before the
  client polls the original SSE stream for another event.
- Repeated and combined `WWW-Authenticate` fields; quoted commas; 401 failure
  mapping; 403 insufficient-scope mapping; agreeing, missing, malformed, and
  conflicting `resource_metadata`; scope deduplication; session-expiry 404;
  `close()` tolerating 405.
- Naming: sanitization, 64-char truncation, collision suffixing, dispatch
  strip-prefix round-trip, `InvalidServerKey`.

Integration tests (crate-root `tests/`, in-process Axum servers, real
`ReqwestMcpHttpTransport`) cover JSON and SSE initialize → list → call flows,
static Bearer authentication, session close, and repeated 401/403 challenges.
The SSE server gates its matching response and EOF on receiving the client's
reply to an interleaved server request, proving that the client processes
events incrementally without deadlock. A credential-free MCP adapter is also
constructed by `cargo xtask smoke-test`. No ignored live test is required
unless a stable public MCP test server becomes available.

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
  a 401 without credentials yields `Error::AuthorizationRequired`, and a 403
  yields `Error::Forbidden`, each carrying raw and typed challenge details.
- No auth flows, storage, or product policy anywhere in the crate.
- All workspace checks pass (`cargo xtask check`), 100% test pass rate.

## Appendix: how the downstream consumer will wire it (context only)

Per configured server: build `ReqwestMcpHttpTransport`, an implementation of
`json_http::JsonHttpAuth` backed by its credential store (refreshing tokens
before expiry inside the hook), and `StreamableHttpMcpClient` with a
`McpServerConfig`. Per agent turn it `McpToolSet::load`s (with its own
caching) and appends the set to the turn's tools. On
`Error::AuthorizationRequired` or an insufficient-scope `Error::Forbidden`,
the host delegates to [`ai-mcp-oauth`](mcp-oauth.md), then retries according
to that contract. None of the interactive flow is this crate's concern.
