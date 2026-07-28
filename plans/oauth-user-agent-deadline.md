# OAuth User-Agent Deadline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure the OAuth host receives an authorization deadline that never
extends beyond either the callback-state lifetime or the manager's user-agent
timeout.

**Architecture:** Keep `OAuthUserAuthorizationRequest::expires_at()` as the
whole-second UNIX deadline for the complete interaction. Derive it from the
earlier whole-second duration while retaining the existing, independently
configured state and Tokio timeout enforcement. This is a reporting correction
only; it does not change when the manager stops waiting or how late callbacks
map to errors.

**Tech Stack:** Rust, Tokio, Unimock, OAuth manager tests, Cargo workspace
checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).
- Preserve both independent configuration fields and all public APIs.
- Use the same whole-second conversion and saturating arithmetic as the
  authorization state tracker.
- Report a deadline no later than either configured lifetime.
- Do not change user-agent wait duration or late-callback error mapping in this
  plan.
- Keep secret values redacted and avoid real clocks or timers in regressions.

## Milestone 1: Report the Effective Interaction Deadline

At the end of this milestone, hosts can rely on the request deadline for both
possible duration orderings without any change to authorization flow behavior.

**Files:**

- Modify: `crates/ai-mcp-oauth/src/manager/authorize.rs`
- Modify: `crates/ai-mcp-oauth/src/user_agent.rs`
- Modify: `crates/ai-mcp-oauth/src/_tests_/manager/mod.rs`
- Add:
  `crates/ai-mcp-oauth/src/_tests_/manager/authorization_deadline_tests.rs`
- Modify: `docs/protocol/mcp-oauth.md`
- Modify: `crates/ai-mcp-oauth/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpOAuthConfig`, `OAuthUserAuthorizationRequest`, and
  `McpOAuthManager`.
- Change only the private deadline value supplied to the existing request
  constructor.

- [x] Record the cycle-3 Codex finding and independently verify the mismatch
      between the public deadline contract and the manager-owned timeout.
- [x] Have Claude Code Fable 5 validate the finding, severity, whole-second
      minimum calculation, deterministic regressions, and implementation scope
      before editing production code.
- [x] Add deterministic manager-boundary regressions for user-agent-timeout
      first and state-lifetime first.
- [x] Run the focused regressions and confirm the user-agent-timeout-first case
      fails for the expected pre-fix deadline.
- [x] Report the earlier whole-second deadline with saturating addition.
- [x] Run the focused manager tests and complete `ai-mcp-oauth` suite until all
      pass.
- [x] Align the public API docs, protocol, and crate README with the effective
      deadline and its whole-second granularity.
- [x] Have Claude Code Fable 5 audit the completed implementation, regressions,
      documentation, arithmetic, and unchanged flow semantics.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit all completed cycle-3 fixes with a Conventional
      Commit title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
