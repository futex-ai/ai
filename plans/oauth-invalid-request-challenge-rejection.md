# OAuth Invalid-Request Challenge Rejection

## Summary

Reject an MCP Bearer `invalid_request` challenge before discovery,
registration, or browser work because a new grant cannot repair a malformed
MCP request. Keep valid authorization, invalid-token, and incremental-scope
flows unchanged.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).

## Milestone 1: Reject Non-Authorizable Invalid Requests

At the end of this milestone, explicit OAuth authorization returns a distinct
typed failure for `InvalidRequest` without invoking any side-effecting
boundary.

- [x] Record the Codex finding and have Claude Code Fable 5 independently
      confirm validity, severity, impact, error contract, and regression
      requirements before editing implementation code.
- [x] Add a failing strict-mock manager regression proving `InvalidRequest`
      returns before discovery, registration, storage, DNS, randomness, clock,
      token transport, or user-agent work.
- [x] Add an integration regression covering the public manager boundary.
- [x] Add a distinct `AuthorizationInvalidRequest` error and reject the
      challenge immediately after context validation.
- [x] Preserve existing behavior for `AuthorizationRequired`, `InvalidToken`,
      `InsufficientScope`, and `Forbidden`.
- [x] Update the OAuth protocol and crate README with challenge eligibility and
      typed failure behavior.
- [x] Run focused OAuth manager and integration tests.
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
