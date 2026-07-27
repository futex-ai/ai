# OAuth Empty Refresh Token Handling

## Summary

Treat an empty `refresh_token` in a successful OAuth token response as absent.
An initial grant then remains usable without refresh capability, while a
refresh response retains the previous valid refresh token instead of replacing
it with an unusable credential.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).

## Milestone 1: Preserve Usable Token Credentials

At the end of this milestone, an empty refresh-token response value cannot be
stored, submitted, or preferred for revocation, and normal omitted or non-empty
refresh-token behavior remains unchanged.

- [x] Record the Codex finding and have Claude Code Fable 5 independently
      confirm validity, calibrated severity, RFC behavior, impact, and the
      exact empty-value policy before editing implementation code.
- [x] Add failing token-response regressions for an empty refresh token with
      and without a previous refresh credential.
- [x] Treat an empty refresh token exactly like an omitted field before secret
      wrapping, retaining a previous token when one exists.
- [x] Preserve coverage for omitted refresh-token retention and non-empty
      refresh-token rotation.
- [x] Update the OAuth protocol and crate README with empty refresh-token
      handling and its RFC rationale.
- [x] Run focused token-endpoint, refresh, disconnect, and OAuth crate tests.
- [x] Have Claude Code Fable 5 audit the completed implementation, tests, and
      documentation and resolve every valid solution issue.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, and untracked files.
- [x] Run `git add -A`, commit the checked work with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [x] Complete the authorized post-push `cargo xtask review` workflow against
      `origin/main` and investigate every finding.
- [x] Once every authorized review finding is resolved, mark this plan
      completed in `plans/README.md` and commit and push the final bookkeeping
      without exceeding the ten-cycle review limit.
