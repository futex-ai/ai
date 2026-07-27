# MCP JSON-RPC Response Exclusivity

## Summary

Reject inbound JSON-RPC response objects that contain both `result` and
`error`. Such a response violates the JSON-RPC 2.0 response contract and must
take the existing malformed-message path instead of being accepted as a
successful result.

The source of truth is
[`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).

## Milestone 1: Reject Ambiguous Response Outcomes

At the end of this milestone, buffered JSON and SSE responses enforce mutual
exclusivity between success and error members while all valid message
classification remains unchanged.

- [x] Record the Codex cycle-10 finding and independently inspect the shared
      classifier, response consumers, test coverage, protocol documentation,
      and JSON-RPC 2.0 response requirements.
- [x] Have Claude Code Fable 5 validate the finding, severity, relationship to
      the prior classifier scope, minimal implementation boundary, regression
      coverage, and documentation scope before editing implementation code.
- [x] Add a failing classifier regression proving a response with both
      `result` and `error` is invalid for valid, null, absent, and wrong-typed
      identifiers.
- [x] Add failing buffered-JSON and SSE client regressions proving a mixed
      response returns the existing typed malformed-message error rather than
      a successful result or typed remote error.
- [x] Reject mixed `result`/`error` response objects once at the shared
      classifier without changing public protocol types.
- [x] Preserve valid success responses, valid and null-id error responses,
      request/notification classification, identifier validation, and strict
      JSON-RPC version validation.
- [x] Update the MCP protocol and crate README with response-member
      exclusivity and malformed-response behavior.
- [x] Run focused classifier, JSON client, SSE client, and complete MCP crate
      tests.
- [x] Have Claude Code Fable 5 audit the implemented solution, regressions,
      and documentation; resolve every valid issue before repository gates.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, and untracked files.
- [ ] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters, and push the current branch.
- [x] Record cycle 10 as the final authorized `cargo xtask review`; do not
      start an eleventh review cycle.
- [ ] Once the cycle-10 finding is resolved and Fable passes, mark this and all
      remaining review-fix plans completed in `plans/README.md`, then commit and
      push the final bookkeeping without starting an eleventh review cycle.
