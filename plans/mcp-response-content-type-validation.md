# MCP Response Content-Type Validation

## Summary

Enforce the streamable HTTP response media-type contract when a successful MCP
JSON-RPC request response is interpreted. Preserve empty `202` responses,
DELETE behavior, non-success response bodies, and parameterized standard media
types.

The source of truth is
[`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).

## Milestone 1: Validate Successful Request Responses

At the end of this milestone, successful non-empty MCP request responses accept
only JSON or SSE media types and return a typed error for unsupported or
missing content types without weakening existing status/body behavior.

- [x] Record the Codex finding and have Claude Code Fable 5 independently
      confirm validity, severity, impact, implementation boundary, and
      regression requirements before editing implementation code.
- [x] Add failing unit and production-transport regressions for unsupported and
      missing JSON response content types.
- [x] Cover parameterized JSON/SSE media types, empty `202` responses,
      non-success response bodies, and DELETE responses against regressions.
- [x] Add one shared media-type essence classifier and enforce JSON content
      type only when successful JSON-RPC response content is consumed.
- [x] Represent a missing response Content-Type in the typed
      `UnsupportedContentType` error contract.
- [x] Update the MCP protocol and crate README with accepted media types,
      status/body exceptions, and typed failure behavior.
- [x] Run focused MCP unit and integration tests.
- [x] Have Claude Code Fable 5 audit the completed implementation, tests, and
      documentation and resolve every valid solution issue.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, and untracked files.
- [x] Run `git add -A`, commit the checked work with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run `cargo xtask review` against `origin/main` and
      investigate every finding under the active review workflow.
- [ ] Once review has no valid findings, mark this plan completed in
      `plans/README.md`, commit and push the plan bookkeeping, and run the
      final post-push review.
