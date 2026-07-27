# OAuth Unrefreshable Token Expiry Window

## Summary

Keep a stored Bearer access token usable until its actual expiry when the
authorization server did not issue a refresh token. The refresh skew should
trigger an early refresh when refresh is possible; it must not create an
unauthenticated window for an otherwise usable credential.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md). This change is
limited to non-interactive request authentication and does not alter explicit
refresh, interactive authorization, or expired-token behavior.

## Milestone 1: Preserve Usable Unrefreshable Tokens

At the end of this milestone, request authentication uses an unrefreshable
access token through its real expiry, suppresses it once expired, and retains
the existing refresh-skew behavior for refreshable credentials.

- [x] Record the Codex P2 finding and independently confirm its validity,
      severity, exact code path, and minimal solution with Claude Code Fable 5
      before editing implementation code.
- [x] Add a failing auth-hook regression proving a stored, unrefreshable token
      inside the configured refresh-skew window is still returned.
- [x] Add or retain coverage proving a known-expired, unrefreshable token is
      not returned.
- [x] Reuse the existing actual-expiry helper when refresh is impossible,
      without changing explicit-refresh semantics or adding a new error.
- [x] Update the OAuth protocol and crate README to distinguish early refresh
      from actual usability and expiry.
- [x] Run focused token-manager tests and all affected crate tests.
- [x] Have Claude Code Fable 5 audit the completed implementation, tests, and
      documentation and resolve every valid solution issue.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete implementation diff for omissions, unrelated
      changes, Markdown errors, and untracked files.
- [x] Run `git add -A`, commit the checked work with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [x] Complete the authorized post-push `cargo xtask review` workflow against
      `origin/main` and investigate every finding.
- [x] Once every authorized review finding is resolved, mark this plan
      completed in `plans/README.md` and commit and push the final bookkeeping
      without exceeding the ten-cycle review limit.
