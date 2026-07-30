# MCP Authorization Status-Authoritative Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve actionable MCP 401 and 403 challenges without waiting for,
limiting, or decoding response bodies the client does not use.

**Architecture:** Define one crate-private authorization-status predicate
shared by the reqwest transport and client error mapping. For exactly 401 and
403, the production transport captures normalized headers and returns
`McpHttpPayload::None` without polling the body. Every other response retains
its existing bounded decoding and status behavior.

**Tech Stack:** Rust, reqwest, Axum integration tests, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Preserve all public transport, client, challenge, and error APIs.
- Treat exactly 401 and 403 as status/header-authoritative.
- Preserve every repeated `WWW-Authenticate` value in wire order.
- Preserve bounded bodies for every non-authorization response.
- Do not add automatic authorization, retry, or session replay behavior.

## Milestone 1: Surface Challenges Before Reading Bodies

At the end of this milestone, POST and DELETE authorization failures return
their existing typed challenges as soon as headers arrive, even when the
ignored body is oversized, broken, SSE-typed, or long-lived.

**Files:**

- Modify: `crates/ai-mcp/src/authorization.rs`
- Modify: `crates/ai-mcp/src/client_response.rs`
- Modify: `crates/ai-mcp/src/config.rs`
- Modify: `crates/ai-mcp/src/transport/reqwest.rs`
- Modify: `crates/ai-mcp/tests/authorization_transport_tests.rs`
- Modify: `crates/ai-mcp/tests/delete_transport_tests.rs`
- Modify: `crates/ai-mcp/tests/support/mod.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpHttpTransport`, `McpHttpResponse`, and `McpClient`.
- Add only a crate-private status predicate and test support.

- [x] Record the cycle-7 Codex finding and independently trace response
      decoding, client error precedence, challenge parsing, and every payload
      consumer.
- [x] Verify the header-authoritative flow against MCP 2025-06-18 and RFC 9728.
- [x] Have Claude Code Fable 5 validate the finding, P2 severity, exact
      401/403 predicate, unread-body behavior, test matrix, and documentation
      scope before editing production code.
- [x] Add failure-first real-transport client regressions for oversized 401
      and 403 POST bodies that preserve typed challenges and repeated headers.
- [x] Add failure-first direct transport coverage proving an oversized 401
      POST returns status, headers, and `McpHttpPayload::None`.
- [x] Add failure-first DELETE 401 coverage through the production transport.
- [x] Confirm all new regressions fail through `ResponseTooLarge` before the
      production fix.
- [x] Share one authorization-status predicate between transport and client.
- [x] Return status, normalized headers, and `McpHttpPayload::None` before
      SSE classification or body polling for exactly 401 and 403.
- [x] Run focused authorization/DELETE tests and the complete `ai-mcp` suite.
- [x] Align the protocol, crate README, and integration-test inventory with
      status/header-authoritative authorization responses.
- [x] Have Claude Code Fable 5 audit the implementation, regressions,
      documentation, and preserved non-authorization behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [ ] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding within the invocation's
      ten-cycle limit.
