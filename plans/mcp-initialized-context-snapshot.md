# MCP Initialized Context Snapshot Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent concurrent session invalidation from turning an otherwise
valid MCP operation into a synthetic initialization `MissingResponse`.

**Architecture:** Introduce one private initialization helper that returns the
handshake and its matching `RequestContext` from the same state snapshot.
`ensure_initialized()` projects the public handshake, while `list_tools()` and
`call_tool()` use the context returned by that helper. If invalidation happens
after the snapshot, the operation retains its captured session and its own
server response determines whether it receives `SessionExpired`; do not replay
an already-dispatched operation.

**Tech Stack:** Rust, Tokio mutex scheduling, deterministic client unit tests,
Unimock-compatible trait boundaries, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Preserve the public `McpClient` trait and all public error variants.
- Never return `MissingResponse` merely because another request invalidated
  cached state before this operation dispatched.
- Do not automatically replay a request that was sent with an expired session.
- Preserve compare-and-clear protection for late responses and session DELETE.
- Avoid retry loops and keep initialization single-flight.

## Milestone 1: Couple Initialization and Context Capture

At the end of this milestone, tool-list and tool-call operations capture a
matching initialized context without a second fallible state lookup.

**Files:**

- Modify: `crates/ai-mcp/src/client_operations.rs`
- Modify: `crates/ai-mcp/src/_tests_/client/mod.rs`
- Add: `crates/ai-mcp/src/_tests_/client/context_invalidation_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpClient::ensure_initialized`, `list_tools`, and `call_tool`.
- Add only private initialized-state snapshot helpers.

- [x] Record the cycle-5 Codex finding and independently trace the race across
      the two state-lock acquisitions.
- [x] Have Claude Code Fable 5 validate the finding, severity, atomic-snapshot
      solution, deterministic mutex-queue regression, and no-replay semantics
      before editing production code.
- [x] Add a deterministic failure-first regression that queues an operation,
      matching 404 invalidation, and second context lookup in that order.
- [x] Confirm the regression returns initialization `MissingResponse` before
      the fix instead of the operation's own `SessionExpired`.
- [x] Return the handshake and matching context from one initialized-state
      snapshot and use it in both list and call operations.
- [x] Run the focused concurrency tests and complete `ai-mcp` suite.
- [x] Align protocol and crate README concurrency semantics.
- [x] Have Claude Code Fable 5 audit the implementation, regression,
      documentation, and unchanged request replay/session invalidation rules.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [x] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
