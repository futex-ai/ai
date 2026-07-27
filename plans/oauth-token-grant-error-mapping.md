# OAuth Token-Grant Error Mapping

## Summary

Preserve token endpoint `invalid_grant` as a status-bearing rejection for an
authorization-code exchange while retaining stale-refresh cleanup for refresh
requests. The shared wire error must not imply refresh-token recovery outside
the refresh path.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).

## Milestone 1: Keep Recovery Grant-Aware

At the end of this milestone, authorization-code rejection remains a typed
token endpoint failure, refresh-token rejection still removes stale local
credentials, and no refresh-only public error is emitted from the wrong path.

- [x] Record the Codex finding and independently inspect the parser, both
      callers, public error contract, recovery behavior, and RFC semantics.
- [x] Have Claude Code Fable 5 validate the finding, severity, impact,
      solution boundary, regression matrix, and documentation scope before
      editing implementation code.
- [x] Add a failing parser regression proving authorization-code
      `invalid_grant` preserves HTTP status in
      `TokenRejected { error: OAuthTokenError::InvalidGrant }`.
- [x] Add a failing manager regression proving authorization-code rejection
      does not delete or replace stored credentials.
- [x] Preserve all token endpoint errors as `TokenRejected`, match the typed
      `InvalidGrant` wire error only inside refresh recovery, and remove the
      unobservable refresh-only public error variant.
- [x] Retain explicit-refresh and auth-hook cleanup coverage for a rejected
      refresh token.
- [x] Update the OAuth protocol and crate-facing error documentation to
      distinguish authorization-code rejection from refresh recovery.
- [x] Run focused token, authorization, refresh, and OAuth crate tests.
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
