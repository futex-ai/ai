# AI MCP Crate

## Summary

Add `crates/ai-mcp` as a protocol-focused Model Context Protocol client for
the 2025-06-18 streamable HTTP transport, with negotiated support for
2025-03-26, and expose discovered server tools through
`ai_interface::Tool` for use by `ai-tool-calling`.

The approved and durable source of truth is
[`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md). Implementation must
keep that protocol document, the crate README, tests, and public API aligned.
OAuth flows, credential persistence, token refresh, product policy, stdio,
legacy HTTP+SSE, and all non-tool MCP capabilities remain out of scope.
The follow-on [`AI MCP OAuth`](ai-mcp-oauth.md) plan consumes this crate's
typed authorization challenges and auth hook without moving those concerns
into `ai-mcp`.

## Milestone 1: Streamable HTTP Protocol Client

Create a usable protocol client with a mockable HTTP seam. At the end of this
milestone, a caller can initialize a supported MCP server, list all of its
tools, call a tool by its original name, inspect tool-list invalidation, and
close the session without depending on the tool adapter.

- [x] Verify the approved transport, schema, tools, and authorization
      requirements against the versioned official MCP 2025-06-18
      specification before coding; if an official requirement would change
      this approved contract, record the discrepancy and request protocol
      direction first.
- [x] Add `crates/ai-mcp` to the workspace members and add
      `ai-mcp = { version = "0.2.0", path = "crates/ai-mcp" }` to workspace
      dependencies.
- [x] Create the crate manifest with workspace package metadata, the
      `test-support` feature, workspace-internal `ai-interface` and
      `json-http` dependencies, and the approved external dependencies; use
      `cargo add` for every crates.io dependency and configure the required
      `reqwest` and Tokio features. Add `unimock` as an optional normal
      dependency wired through `test-support = ["dep:unimock"]` and as a
      dev-dependency for ordinary tests, matching `json-http`.
- [x] Keep `lib.rs` as a thin documented module root, add
      `#![warn(unreachable_pub)]`, use narrow visibility, and export only the
      intentional config, error, protocol, transport, client, and adapter
      contracts.
- [x] Add `McpServerConfig`, its documented defaults, and validation for the
      `[a-z0-9_-]{1,32}` server-key contract without introducing product or
      credential state.
- [x] Define the typed `ai_mcp::{Error, Result}` contract, including actionable
      authorization challenges, expired sessions, unsupported versions,
      JSON-RPC errors, missing responses, HTTP failures, response-size limits,
      deserialization, transport, auth-hook, and invalid-key failures.
- [x] Add `McpAuthorizationChallenge` and `McpAuthorizationFailure`; expose
      distinct `AuthorizationRequired` (401) and `Forbidden` (403) errors with
      raw header values, agreed RFC 9728 `resource_metadata`, recognized Bearer
      failure, description, and deduplicated scope hints.
- [x] Define fully typed serde DTOs for JSON-RPC envelopes, initialization,
      server capabilities and identity, tool descriptors, pagination,
      tool-call outcomes, annotations, embedded text/blob resources, and every
      known MCP content block; preserve unrecognized content block objects
      exactly, and convert raw JSON at the protocol boundary rather than
      carrying untyped envelopes through client logic.
- [x] Add failing serde tests for every known content-block field and
      camel-case wire name, optional annotations and metadata, embedded
      text/blob resource selection, omitted `isError`, malformed known blocks,
      and exact unknown-block round trips before implementing those DTOs.
- [x] Implement the public `McpServerHandshake`, `McpServerInfo`,
      `McpServerCapabilities`, and `McpToolsCapability` contract, projecting
      negotiated version, server identity, optional instructions, and the
      tools/list-changed capability while ignoring non-goal capabilities.
- [x] Define the single-owner `McpEventStream` boundary and boxed dyn alias so
      the HTTP transport can return response status and headers immediately
      while the client pulls decoded SSE messages before EOF.
- [x] Add failing source-adjacent unit tests for chunk-split SSE event framing,
      multi-line `data:` fields, ignored fields, JSON decoding, yielding a
      completed event before EOF, and cumulative `max_response_bytes`
      enforcement before implementing the parser.
- [x] Implement the trait-backed `McpHttpTransport` boundary and
      `ReqwestMcpHttpTransport` for POST and DELETE, receiving completed header
      maps from the client, normalizing response-header names, buffering capped
      JSON bodies, returning live pull-based SSE bodies without waiting for
      EOF, preserving every repeated response-header value in wire order,
      passing the configured response limit into both methods, and enforcing
      timeouts and cumulative byte limits while reading.
- [x] Export Unimock-generated transport, event-stream, and client mocks only
      for tests, doctests, or the `test-support` feature, following the
      `json-http` pattern.
- [x] Add failing unit tests for initialization idempotence, supported and
      unsupported version negotiation, monotonically increasing request IDs,
      exact preservation of string and numeric incoming server-request IDs,
      server identity/capability/instructions projection, omitted
      `listChanged` defaulting to false, session capture/replay,
      protocol-version headers, initialized notification acceptance, and safe
      concurrent calls.
- [x] Implement `StreamableHttpMcpClient` behind the `McpClient` trait with
      synchronized initialization/session state, collision-free request IDs,
      automatic tool-list pagination, original-name tool calls, stale-list
      tracking, and DELETE close semantics including tolerated 405 responses;
      build protocol/session headers, apply the injected `JsonHttpAuth` hook,
      and pass the completed map to the transport before every request.
- [x] Add failing unit tests and then implement SSE side-message handling:
      mark `notifications/tools/list_changed`, reply to `ping` with an empty
      result, reply to unsupported server requests with JSON-RPC `-32601`,
      ignore other notifications, complete each response POST before polling
      the original stream again, and require the response matching the posted
      request ID.
- [x] Test repeated and combined `WWW-Authenticate` fields, quoted commas,
      agreeing/missing/malformed/conflicting RFC 9728 `resource_metadata`,
      401 invalid-token mapping, 403 insufficient-scope mapping, scope
      deduplication, session-expiry 404 behavior only when a session header was
      sent, other HTTP statuses, malformed responses, auth-hook failure,
      response overflow, and close success/failure paths.
- [x] Add the crate `README.md` with the required Responsibilities, What This
      Crate Does, Quick Start, Development, Key Code, and Related Docs
      sections, including unauthenticated and bearer-auth examples.
- [x] Run `cargo fmt --all -- --check`, targeted Clippy with warnings denied,
      `cargo test -p ai-mcp --all-features`, and
      `cargo xtask rust-file-length-lint --all`, followed by
      `cargo xtask check`; fix failures until this milestone is green.
- [x] Run `git add -A`, commit the green protocol-client milestone with a
      descriptive Conventional Commit whose title is at most 50 characters,
      and push the current branch.
- [x] After the push, run `cargo xtask review`; do not apply findings
      automatically, and report them with the required severity, context,
      impact, solution options, and recommendation for user decision.

## Milestone 2: Tool Adapter And Runtime Dispatch

Expose a stable snapshot of one MCP server's tools through the shared tool
boundary. At the end of this milestone, `ai-tool-calling` can register the
adapter, advertise provider-legal definitions, and dispatch model tool calls
back to the correct original MCP tool.

- [x] Implement `McpToolSet::new` and `McpToolSet::load` with the protocol's
      borrowed `&McpServerConfig` signature, copying only the server key,
      activity verb, and response-size limit needed by the immutable adapter
      snapshot.
- [x] Add failing unit tests for deterministic `mcp__{server_key}__{tool}`
      naming, character replacement, 64-character truncation, collision
      suffixes with re-truncation, stable prefixed-to-original dispatch, and
      invalid server keys before implementing the naming helpers.
- [x] Build `ToolDefinition` snapshots from discovered descriptors using the
      required description/title/name fallback, pass through input schemas,
      attach the optional activity verb, and report group `mcp` only for names
      owned by this adapter.
- [x] Add failing unit tests for unknown names, client failures, MCP
      `is_error` outcomes, structured content, single text results,
      multi-block wire JSON retaining annotations, metadata, embedded
      resources, and unknown blocks, plus UTF-8-safe truncation envelopes
      before implementing `Tool::call`.
- [x] Implement the load/new snapshot APIs and `ai_interface::Tool` adapter
      with the exact load-bearing result precedence from the protocol spec;
      keep MCP tool-execution errors as successful model-visible envelopes and
      convert only protocol/client failures into `ToolError::Execution`.
- [x] Add a credential-free runtime smoke test that registers `McpToolSet` as
      an `ai_interface::DynTool` in `ai-tool-calling`, advertises its
      definitions, dispatches a prefixed name, and observes the remote result.
- [x] Update the crate README with adapter construction, refresh ownership,
      naming, output mapping, and downstream runtime registration examples.
- [x] Run formatting, targeted Clippy with warnings denied, all `ai-mcp`
      feature combinations, the targeted runtime smoke test, and the Rust
      file-length lint, followed by `cargo xtask check`; fix failures until
      this milestone is green.
- [x] Run `git add -A`, commit the green tool-adapter milestone with a
      descriptive Conventional Commit whose title is at most 50 characters,
      and push the current branch.
- [x] After the push, run `cargo xtask review`; do not apply findings
      automatically, and report them with the required severity, context,
      impact, solution options, and recommendation for user decision.

## Milestone 3: Integration Hardening And Completion

Validate the production transport against realistic servers and finish the
workspace integration. At the end of this milestone, JSON and SSE servers,
static auth, session handling, and tool dispatch are covered end to end; all
workspace checks pass and the completed change is pushed for independent AI
review.

- [x] Add crate-root Axum integration-test support with an in-process server
      that records headers and JSON-RPC messages without using external
      credentials or network services.
- [x] Add a JSON-response integration flow covering initialize,
      `notifications/initialized`, paginated list, call, session/protocol
      headers, auth application, and close through the real reqwest transport.
- [x] Add the equivalent SSE-response integration flow, including interleaved
      notifications and server requests; gate the matching final response and
      EOF on receipt of the client's side-request reply so the test fails if
      the implementation buffers the original stream to completion.
- [x] Add end-to-end 401 and 403 challenge cases that assert raw header
      preservation and typed discovery/scope details, plus an authenticated
      success case using `StaticHeaderAuth::bearer_token`.
- [x] Add a credential-free smoke-test entry to the workspace automation; add
      an ignored `AI_MCP_SMOKE_URL` live test only if a stable manual test
      server is available, and document exactly how to run it.
- [x] Audit every changed Rust module for module/public API documentation,
      trait-backed impure boundaries, typed error propagation, production
      panic/`unwrap`/`expect`/`map_err` use, import order, test placement, and
      the 300-line hard cap.
- [x] Update the root README feature, interface, and key-code maps for
      `ai-mcp`; reconcile the crate README and protocol document with the
      implemented public behavior and capture any useful integration context.
- [x] Run `cargo fmt --all -- --check`,
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      `cargo test --workspace --all-features`,
      `cargo xtask rust-file-length-lint --all`, the credential-free smoke
      tests, and `cargo xtask check`; keep working until every required check
      passes with a 100% test pass rate.
- [x] Mark every completed TODO and milestone in this plan, then move its link
      from Active to Completed in `plans/README.md`.
- [x] Review `git diff origin/main...` and `git status` for unrelated changes,
      missing/new untracked files, generated artifacts, and stale references.
- [x] Run `git add -A`, commit all completed work with a descriptive
      Conventional Commit whose title is at most 50 characters, and push the
      current branch.
- [x] After the push, run `cargo xtask review` against `origin/main`; do not
      change review findings automatically, and report every finding with a
      number, severity, feature/codebase context, impact of doing nothing,
      lettered solution options, and a recommended option for user decision.
