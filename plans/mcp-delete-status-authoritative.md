# MCP Status-Authoritative DELETE Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make accepted MCP session DELETE statuses authoritative without
waiting for, limiting, or decoding a response body that the client ignores.

**Architecture:** Define one private tolerated-DELETE predicate shared by the
reqwest transport and `close()`. The production DELETE transport returns a
status/header-only `McpHttpResponse` for 2xx and 405, dropping the unread body.
All other statuses retain the existing bounded body decoder so session expiry,
authorization failures, redirects, and HTTP diagnostics remain unchanged.

**Tech Stack:** Rust, reqwest, Axum integration tests, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Preserve all public client and transport traits and error variants.
- Treat exactly 2xx and 405 as accepted close statuses in both layers.
- Preserve bounded decoding for every non-accepted DELETE response.
- Preserve session compare-and-clear behavior and never replay DELETE.

## Milestone 1: Stop Reading Accepted DELETE Bodies

At the end of this milestone, accepted session termination returns as soon as
headers arrive and clears only the captured current session.

**Files:**

- Add: `crates/ai-mcp/src/transport/delete_status.rs`
- Modify: `crates/ai-mcp/src/transport/mod.rs`
- Modify: `crates/ai-mcp/src/transport/reqwest.rs`
- Modify: `crates/ai-mcp/src/client_operations.rs`
- Add: `crates/ai-mcp/tests/delete_transport_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpHttpTransport::delete` and `McpClient::close`.
- Add only crate-private status classification and response-decoding helpers.

- [x] Record the cycle-6 Codex finding and independently trace the body read
      that precedes close status handling.
- [x] Have Claude Code Fable 5 validate the finding, P2 severity, exact
      2xx/405 predicate, unread-body behavior, regression design, and
      non-success preservation before editing production code.
- [x] Add failure-first production-transport coverage for oversized 2xx and
      405 DELETE bodies.
- [x] Add a failure-first client regression proving successful close clears
      session state despite an oversized ignored body.
- [x] Confirm both regressions fail through `ResponseTooLarge` before the fix.
- [x] Share one tolerated DELETE predicate between transport and client.
- [x] Return status, normalized headers, and `McpHttpPayload::None` without
      reading accepted DELETE bodies.
- [x] Run focused DELETE tests and the complete `ai-mcp` suite.
- [x] Align protocol and crate README body-limit semantics.
- [x] Add the Fable-recommended DELETE integration-suite references to the
      crate development guide and protocol verification inventory.
- [x] Have Claude Code Fable 5 audit the implementation, regressions,
      documentation, and preserved non-success/session behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [x] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
