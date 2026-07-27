# OAuth Transient Invalid-Grant Preservation

## Summary

Delete stale refresh credentials only when an RFC-compliant token endpoint
rejects the refresh token with HTTP 400 and `invalid_grant`. A transient or
non-conforming response that carries the same JSON error at another status
must preserve local credentials and remain a status-bearing token rejection.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).

## Milestone 1: Make Refresh Cleanup Status-Aware

At the end of this milestone, explicit and request-hook refreshes recover from
an authoritative stale-token rejection while never turning HTTP 5xx failures
into a permanent local disconnect.

- [x] Record the Codex cycle-8 finding and independently inspect both refresh
      entrypoints, the shared token parser, storage side effects, existing
      regressions, and RFC 6749 token-error status requirements.
- [x] Have Claude Code Fable 5 validate the finding, calibrated severity,
      exact HTTP-status boundary, minimal implementation, test matrix, and
      documentation scope before editing implementation code.
- [x] Add failing strict-store regressions proving HTTP 503
      `invalid_grant` preserves credentials for explicit refresh and request
      authentication while surfacing the original `TokenRejected`.
- [x] Retain coverage proving HTTP 400 `invalid_grant` deletes stale tokens
      and produces the existing explicit-refresh and auth-hook recovery
      outcomes.
- [x] Require HTTP 400 alongside the typed `InvalidGrant` match in both
      refresh-only recovery paths without changing the shared parser or
      authorization-code exchange.
- [x] Update the OAuth protocol and crate README to make the status-qualified
      cleanup contract explicit.
- [x] Run focused refresh, request-token, token-endpoint, and OAuth crate
      tests.
- [x] Have Claude Code Fable 5 audit the implemented solution, regressions,
      and documentation; resolve every valid issue before repository gates.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, and untracked files.
- [ ] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters, and push the current branch.
- [ ] After the push, run `cargo xtask review` against `origin/main` and
      investigate every finding under the active review workflow.
- [ ] Once review has no valid findings, mark this plan completed in
      `plans/README.md`, commit and push the bookkeeping, and run the final
      post-push review.
