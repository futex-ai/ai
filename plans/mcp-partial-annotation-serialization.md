# MCP Partial Annotation Serialization Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the exact optional-member shape of known MCP content
annotations when tool results are converted back to model-visible JSON.

**Architecture:** Keep `McpAnnotations` as the typed wire representation and
retain the existing content-block serializer. Configure each optional
annotation member to be omitted when absent, matching the omission behavior
already used by optional content-block fields. Cover both direct known-content
round trips and the `McpToolSet` result-mapping boundary.

**Tech Stack:** Rust, Serde, Serde JSON, Unimock, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Preserve all public MCP content types and field names.
- Preserve explicit values, including an empty audience list and numeric zero.
- Omit absent optional members; do not emit synthetic JSON `null` values.
- Treat schema-invalid explicit `null` annotation members as absent and omit
  them when known content is serialized.
- Preserve unknown content blocks exactly as received.
- Keep direct wire serialization and model-visible tool-result mapping aligned.

## Milestone 1: Preserve Partial Annotation Shape

At the end of this milestone, a partial annotations object round-trips without
gaining absent fields and keeps the same shape through tool-result mapping.

**Files:**

- Modify: `crates/ai-mcp/src/protocol/content.rs`
- Modify: `crates/ai-mcp/src/_tests_/mod.rs`
- Add: `crates/ai-mcp/src/_tests_/content_serde_tests.rs`
- Modify: `crates/ai-mcp/src/_tests_/tool_set/call_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpAnnotations`, `McpContentBlock`, and `McpToolSet`.
- Change only the wire serialization of absent annotation members.

- [x] Record the cycle-4 Codex finding and independently trace the unwanted
      `null` members to derived `McpAnnotations` serialization.
- [x] Have Claude Code Fable 5 validate the finding, severity, proposed Serde
      omission fix, regression scope, and compatibility impact before editing
      production code.
- [x] Add a failure-first known-content round-trip regression for an
      audience-only annotations object.
- [x] Add a failure-first tool-result regression proving model-visible
      multi-block and remote-error content do not gain absent annotation
      members.
- [x] Run the focused regressions and confirm they fail for the expected
      synthetic `priority` and `lastModified` nulls.
- [x] Omit each absent `McpAnnotations` member during serialization while
      preserving present values.
- [x] Run focused protocol and tool-set tests and the complete `ai-mcp` suite.
- [x] Align the protocol and crate README with the partial-annotation
      serialization contract, including explicit-null normalization.
- [x] Have Claude Code Fable 5 audit the completed implementation,
      regressions, documentation, and unchanged known/unknown content behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit all completed cycle-4 fixes with a Conventional
      Commit title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
