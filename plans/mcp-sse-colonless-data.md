# MCP Colonless SSE Data Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply WHATWG field parsing to colonless SSE `data` lines so an empty,
malformed MCP JSON-RPC event cannot be silently ignored.

**Architecture:** Parse each SSE line at its first colon when present;
otherwise treat the complete line as the field name with an empty value.
Continue ignoring non-`data` fields. A colonless `data` field then contributes
an empty data value and follows the existing JSON parsing path, producing
`Error::DeserializeResponse` for an empty payload.

**Tech Stack:** Rust, incremental SSE decoder tests, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).
- Preserve CRLF, CR, and LF framing behavior and cumulative byte limits.
- Preserve one-optional-space stripping after a colon.
- Ignore colonless non-`data` fields exactly as other unsupported SSE fields.
- Fail malformed `data` payloads before later messages or side effects.

## Milestone 1: Parse Colonless Fields Correctly

At the end of this milestone, `data` without a colon is handled as an empty
SSE data value and rejected as malformed MCP JSON, while other fields remain
ignored.

**Files:**

- Modify: `crates/ai-mcp/src/transport/sse.rs`
- Modify: `crates/ai-mcp/src/_tests_/sse_tests.rs`
- Modify: `docs/protocol/ai-mcp.md`
- Modify: `crates/ai-mcp/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `McpEventStream`, `ReqwestEventStream`, and every public error.
- Change only private SSE field parsing.

- [x] Record the cycle-5 Codex finding and independently confirm WHATWG
      colonless-field semantics and the current silent-ignore path.
- [x] Have Claude Code Fable 5 validate the finding, severity, line-parser
      solution, regression scope, and side-effect ordering before editing
      production code.
- [x] Add a failure-first decoder regression proving `data` without a colon is
      not ignored before a later valid event.
- [x] Confirm the regression accepts the later event before the fix instead of
      returning `Error::DeserializeResponse`.
- [x] Treat colonless fields as empty-valued fields and pass colonless `data`
      through the existing JSON decoder.
- [x] Run focused SSE tests and the complete `ai-mcp` suite.
- [x] Align protocol and crate README SSE field semantics.
- [x] Have Claude Code Fable 5 audit the implementation, regression,
      documentation, and unchanged framing/size/side-effect behavior.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [x] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
