# MCP Error SSE Body Decoding Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve bounded diagnostics when an ordinary MCP HTTP error carries
an SSE media type without treating that error body as a live response stream.

**Architecture:** Keep authorization responses and session-bound 404s
status/header-authoritative. For every other response, create a live SSE
payload only when the status is successful; buffer and cap non-success bodies
through the existing lenient JSON/text decoder. Accepted session DELETE
responses remain bodyless before the shared decoder.

**Tech Stack:** Rust, reqwest, Axum integration tests, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Preserve every public transport, response, client, and error API.
- Preserve lazy bounded streaming for successful SSE request responses.
- Preserve bodyless 401/403, session-bound 404, and accepted DELETE handling.
- Preserve bounded JSON or textual diagnostics for every other non-success
  POST and DELETE response, independent of its advertised media type.
- Do not decode SSE framing as protocol messages after HTTP status has already
  classified the response as an HTTP error.

## Milestone 1: Buffer Ordinary SSE-Typed Errors

At the end of this milestone, ordinary SSE-typed HTTP errors follow the same
bounded diagnostic path as other error bodies while successful SSE responses
remain incrementally streamed.

**Files:**

- Modify: `crates/ai-mcp/src/transport/reqwest.rs`
- Add: `crates/ai-mcp/tests/error_sse_transport_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpHttpTransport`, `McpHttpResponse`, `McpHttpPayload`, and
  `McpClient`.
- Change only the private reqwest response-classification predicate.

- [x] Record the cycle-9 Codex finding and independently trace ordinary POST,
      side-response POST, DELETE, transport decoding, HTTP error mapping, and
      payload conversion.
- [x] Verify the intended behavior against the local MCP protocol and the MCP
      2025-06-18 Streamable HTTP response contract.
- [x] Have Claude Code Fable 5 validate the finding, severity, exact
      successful-status streaming boundary, affected request paths,
      regression matrix, and preserved behaviors before editing production
      code.
- [x] Add failure-first production-client coverage that preserves the raw
      bounded diagnostic for a small SSE-typed 429 response.
- [x] Add the explicit failure-first non-session SSE-typed 404 diagnostic case
      recommended by Fable 5.
- [x] Add failure-first direct transport coverage proving oversized ordinary
      SSE-typed POST errors return `ResponseTooLarge`.
- [x] Add failure-first direct transport coverage proving small and oversized
      non-tolerated SSE-typed DELETE errors use bounded buffered decoding.
- [x] Confirm the diagnostic and size-limit regressions fail through the old
      live-stream classification before production changes.
- [x] Restrict live `McpHttpPayload::EventStream` construction to successful
      SSE-typed responses after the existing authoritative-status checks.
- [x] Run the focused error-SSE transport tests and complete `ai-mcp` suite.
- [x] Align the protocol, crate README, and integration-test inventory with
      status-aware SSE payload classification.
- [x] Have Claude Code Fable 5 audit the implementation, regressions,
      documentation, and preserved success/authorization/session/DELETE
      behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [ ] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the final `cargo xtask review` cycle against
      `origin/main`; investigate every finding within the invocation's
      ten-cycle limit.
