# MCP Error Truncation Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep oversized MCP tool-execution failures visibly marked as errors
while enforcing the configured model-visible response limit.

**Architecture:** Preserve the existing success truncation envelope and add an
error-specific envelope with `is_error: true`. Bound a serialized prefix of
the remote error content, raise the global minimum to the larger empty error
envelope, and keep result-precedence behavior unchanged.

**Tech Stack:** Rust, Tokio tests, Serde JSON, `ai-interface`, Cargo workspace
checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Remote MCP tool failures remain `Ok` model-visible values, not local
  `ToolError` failures.
- Every accepted output must serialize within `max_response_bytes`.
- Success truncation must not gain an `is_error` member.
- The empty error truncation envelope is 47 serialized bytes.

## Milestone 1: Preserve Failure Semantics Under Truncation

At the end of this milestone, oversized remote failures retain a stable error
marker and every accepted response limit can contain their empty envelope.

**Files:**

- Modify: `crates/ai-mcp/src/tool_set_result.rs`
- Modify: `crates/ai-mcp/src/_tests_/tool_set_result_tests.rs`
- Modify: `crates/ai-mcp/src/_tests_/tool_set/call_tests.rs`
- Modify: `crates/ai-mcp/src/_tests_/config_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`

**Interfaces:**

- Preserve `map_outcome(&str, McpToolCallOutcome, usize) -> ToolResult<Value>`.
- Keep `Error::InvalidResponseLimit { minimum }`; only its reported minimum
  changes through the shared constant.
- Produce error truncation values shaped as
  `{"is_error":true,"truncated":true,"content":"<prefix>"}`.

- [x] Record the cycle-1 Codex finding and have Claude Code Fable 5 validate
      severity, response-shape semantics, minimum size, regressions, and scope.
- [x] Add failure-first adapter tests for an oversized remote error, UTF-8-safe
      error truncation, exact and nearby 47-byte bounds, absence of
      `is_error` on success truncation, and unchanged under-limit error output.
- [x] Extend the pure envelope drift test to assert both the 31-byte success
      baseline and 47-byte error baseline.
- [x] Update config boundary tests so 46 bytes fails with
      `InvalidResponseLimit { minimum: 47 }` and 47 bytes succeeds.
- [x] Run the focused tests and confirm the new error-marker and minimum
      assertions fail for the expected pre-fix behavior.
- [x] Parameterize truncation by failure semantics, serialize only remote
      error content into the prefix, preserve `is_error: true`, and retain the
      existing success shape and precedence ordering.
- [x] Raise the shared accepted response minimum to 47 without changing the
      transport-side enforcement or tool-output windowing layer.
- [x] Run focused tool-adapter, config, and complete `ai-mcp` tests until all
      pass.
- [x] Align the MCP protocol and crate README with error-specific truncation,
      the 47-byte floor, and unchanged success behavior.
- [x] Have Claude Code Fable 5 audit the completed implementation, regression
      strength, documentation, and adjacent output-management behavior.
- [ ] Run the shared repository verification, commit/push, and post-push
      `cargo xtask review` tasks recorded in all three cycle-1 fix plans.
