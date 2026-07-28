# OAuth Status-Authoritative Revocation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make successful RFC 7009 revocation status authoritative without
waiting for, limiting, or decoding the ignored response body.

**Architecture:** In the production OAuth response decoder, capture status and
normalized headers first. For a revocation endpoint with a 2xx status, return
an `OAuthHttpResponse` with `Value::Null` immediately and drop the unread body.
Every non-2xx revocation response and every other endpoint retains the existing
bounded decoder and typed status/body behavior.

**Tech Stack:** Rust, reqwest, Axum transport tests, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).
- Preserve all public OAuth traits, DTOs, and error variants.
- Treat exactly revocation 2xx responses as status-authoritative.
- Preserve bounded decoding for non-2xx revocation responses and all other
  endpoint responses.
- Preserve unconditional local token deletion and disconnect error precedence.

## Milestone 1: Stop Reading Successful Revocation Bodies

At the end of this milestone, a successful revocation returns after headers
even when the server advertises an oversized or continuing response body.

**Files:**

- Modify: `crates/ai-mcp-oauth/src/transport/reqwest/response.rs`
- Modify: `crates/ai-mcp-oauth/src/_tests_/transport_body_tests.rs`
- Modify: `docs/protocol/mcp-oauth.md`
- Modify: `crates/ai-mcp-oauth/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `OAuthHttpTransport`, `OAuthHttpResponse`, and disconnect APIs.
- Change only private production response decoding.

- [x] Record the cycle-6 Codex finding and independently trace the revocation
      body read that precedes status handling.
- [x] Have Claude Code Fable 5 validate the finding, P2 severity, exact
      revocation-2xx predicate, unread-body behavior, regression design, and
      non-success preservation before editing production code.
- [x] Add a failure-first production-transport regression for an oversized
      successful revocation response body.
- [x] Confirm the regression fails through `ResponseTooLarge` before the fix.
- [x] Return status, normalized headers, and `Value::Null` without consuming
      successful revocation bodies.
- [x] Run focused response-body tests and the complete `ai-mcp-oauth` suite.
- [x] Align protocol and crate README body-limit semantics.
- [x] Have Claude Code Fable 5 audit the implementation, regression,
      documentation, and preserved non-success behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [ ] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
