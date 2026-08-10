# MCP JSON-RPC Version Validation

## Summary

Reject inbound MCP messages whose `jsonrpc` member is missing, non-string, or
not exactly `"2.0"`. Apply the check once at the shared classifier so JSON
responses and SSE responses, requests, and notifications use the same strict
protocol boundary.

The source of truth is
[`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).

## Milestone 1: Enforce JSON-RPC 2.0

At the end of this milestone, only JSON-RPC 2.0 messages reach MCP response or
side-message handling, while every valid existing message behaves unchanged.

- [x] Record the Codex finding and independently inspect the classifier,
      shared client boundary, error path, test coverage, JSON-RPC 2.0
      requirements, and MCP base protocol.
- [x] Have Claude Code Fable 5 validate the finding, severity, impact,
      minimal solution, regression matrix, and documentation scope before
      editing implementation code.
- [x] Add failing classifier regressions for missing, wrong-string, and
      non-string versions across success responses, error responses, server
      requests, and notifications.
- [x] Add failing client regressions proving malformed JSON and SSE messages
      use the existing typed invalid-message path and perform no side effects.
- [x] Require the exact string `"2.0"` before classifying any inbound message,
      without expanding into unrelated JSON-RPC structural validation.
- [x] Retain coverage for valid responses and SSE side-message handling.
- [x] Update the MCP protocol and crate README with strict version validation
      and malformed SSE side-message failure behavior.
- [x] Run focused classifier, client, transport, and MCP crate tests.
- [x] Have Claude Code Fable 5 audit the implemented solution, tests, and
      documentation; resolve every valid issue before repository gates.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, and untracked files.
- [x] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters, and push the current branch.
- [x] Complete the authorized post-push `cargo xtask review` workflow against
      `origin/main` and investigate every finding.
- [x] Once every authorized review finding is resolved, mark this plan
      completed in `plans/README.md` and commit and push the final bookkeeping
      without exceeding the ten-cycle review limit.
