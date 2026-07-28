# MCP Close Replacement Session Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent a late DELETE response for an old MCP session from erasing a
replacement session established while `close()` is in flight.

**Architecture:** Retain request-context capture and network concurrency, then
compare the captured session ID with current state before clearing after a
successful or tolerated DELETE. Add a deterministic gated transport regression
for the full stale-close interleaving.

**Tech Stack:** Rust, Tokio synchronization, async trait mocks, Cargo workspace
checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Never hold the client state mutex across the DELETE await.
- Deliberate close does not create a tool-list invalidation.
- Existing 404 expiry mapping, 405 tolerance, and non-success behavior remain
  unchanged.
- Do not serialize all client operations or delete a replacement session.

## Milestone 1: Compare-And-Clear Stale Close Completion

At the end of this milestone, close clears only the session it observed at
entry and concurrent replacement sessions remain locally active.

**Files:**

- Modify: `crates/ai-mcp/src/client_operations.rs`
- Modify: `crates/ai-mcp/src/_tests_/client/support.rs`
- Modify: `crates/ai-mcp/src/_tests_/client/session_expiry_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`

**Interfaces:**

- Preserve `McpClient::close(&self) -> Result<()>`.
- Add only test-private DELETE coordination to `ScriptedTransport`.
- Compare `ClientState::session_id` with the captured
  `RequestContext::session_id` under one mutex guard before resetting state.

- [x] Record the preceding post-merge Codex finding and have Claude Code
      Fable 5 validate the interleaving, severity, minimal compare-and-clear
      solution, deterministic regression, and scope.
- [x] Add test-private DELETE started/release coordination that consumes the
      scripted DELETE response before blocking and never holds a synchronous
      mutex guard across an await.
- [x] Add a failure-first regression for session A close, matching A expiry,
      session B initialization, and late A DELETE success; assert B and its
      handshake remain current.
- [x] Add or strengthen controls proving matching 200 and tolerated 405 closes
      clear state, while existing 404 and 500 behavior stays unchanged.
- [x] Run focused close/session tests and confirm the stale-close regression
      fails because session B is erased.
- [x] Replace the unconditional reset with mutex-atomic session-ID
      compare-and-clear after accepted DELETE status.
- [x] Run focused session, lifecycle, and complete `ai-mcp` tests until all
      pass.
- [x] Clarify the existing protocol and README rule for successful/405 stale
      DELETE completion without changing public API semantics.
- [x] Have Claude Code Fable 5 audit the completed concurrency fix, forced
      ordering, controls, documentation, and adjacent expiry behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [ ] Run `git add -A`, commit all completed cycle-1 fixes with a Conventional
      Commit title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
