# MCP Session 404 Status-Authoritative Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recover expired MCP sessions even when a session-bound 404 response
has an unreadable, oversized, stalled, or SSE-typed body.

**Architecture:** Detect session binding from the outgoing HTTP header map and
share one crate-private 404 predicate with client error mapping. For exactly a
404 response to a request carrying `Mcp-Session-Id`, the reqwest transport
returns status, normalized headers, and `McpHttpPayload::None` before
content-type classification or body polling. Non-session 404 responses retain
bounded diagnostics.

**Tech Stack:** Rust, reqwest, Axum integration tests, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Target the MCP 2025-06-18 and compatible 2025-03-26 session contracts.
- Preserve all public transport, response, client, and error APIs.
- Treat exactly session-bound 404 as status/header-authoritative.
- Detect outgoing `Mcp-Session-Id` names case-insensitively; header presence,
  including an empty value, establishes the transport boundary.
- Preserve bounded bodies and diagnostics for non-session 404 responses.
- Keep typed expiry, matching-session invalidation, and host retry in the
  client; do not add automatic replay or reinitialization.

## Milestone 1: Surface Expiry Before Reading Bodies

At the end of this milestone, session-bound POST, side-response POST, and
DELETE 404s reach existing client expiry handling immediately, while an
ordinary initialization 404 retains its bounded body behavior.

**Files:**

- Add: `crates/ai-mcp/src/transport/session_status.rs`
- Modify: `crates/ai-mcp/src/transport/mod.rs`
- Modify: `crates/ai-mcp/src/transport/reqwest.rs`
- Modify: `crates/ai-mcp/src/client_response.rs`
- Add: `crates/ai-mcp/tests/session_expiry_transport_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpHttpTransport`, `McpHttpResponse`, `McpHttpPayload`, and
  `McpClient`.
- Add crate-private outgoing-session-header detection and a shared
  `status == 404 && request_was_session_bound` predicate.

- [x] Record the cycle-8 Codex finding and independently trace POST,
      side-response POST, DELETE, transport decoding, typed expiry mapping,
      matching-session invalidation, and host retry.
- [x] Verify the request-header-qualified 404 rule against MCP 2025-06-18 and
      the compatible 2025-03-26 transport contract.
- [x] Have Claude Code Fable 5 validate the finding, P2 severity, exact
      session-bound predicate, transport/client ownership, regression matrix,
      documentation scope, and preserved behaviors before editing production
      code.
- [x] Add a failure-first production-client regression that receives an
      oversized session-A 404, surfaces `SessionExpired`, invalidates A, then
      initializes session B without replaying the failed operation.
- [x] Add failure-first direct transport coverage for oversized
      session-bound POST and DELETE 404s, including lowercase
      `mcp-session-id`, normalized response headers, and
      `McpHttpPayload::None`.
- [x] Add failure-first direct transport coverage that classifies an
      SSE-typed session-bound 404 as `McpHttpPayload::None` before content type.
- [x] Add preservation controls proving oversized non-session POST and DELETE
      404s still fail with `ResponseTooLarge`.
- [x] Confirm the client and direct transport regressions fail through the
      old body decoder or old SSE payload before production changes.
- [x] Add the crate-private session-header detector and shared expiry-status
      predicate.
- [x] Short-circuit session-bound 404 decoding after normalized headers and
      before SSE classification or body polling for POST and DELETE.
- [x] Keep client-owned `SessionExpired` mapping and compare-and-clear logic on
      the same shared predicate.
- [x] Run focused session-expiry transport tests and the complete `ai-mcp`
      suite.
- [x] Align the protocol, crate README, and integration-test inventory with
      body-independent session expiry and preserved non-session diagnostics.
- [x] Have Claude Code Fable 5 audit the implementation, regressions,
      documentation, and preserved authorization/DELETE/error behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [ ] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding within the invocation's
      ten-cycle limit.
