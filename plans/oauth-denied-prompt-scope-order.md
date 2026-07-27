# OAuth Denied-Prompt Scope Order

## Summary

Key incremental-consent denial suppression by the semantic OAuth scope set so
the same scopes cannot reopen the user agent merely because a server reorders
them. Preserve first-seen ordering in public scope values and outbound OAuth
parameters.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).

## Milestone 1: Make Denial Identity Order-Insensitive

At the end of this milestone, repeated insufficient-scope challenges with the
same scope set share one denial entry for an authorization attempt, while
different scope sets and different attempts retain independent behavior.

- [x] Record the Codex finding and have Claude Code Fable 5 independently
      confirm validity, calibrated severity, RFC behavior, implementation
      boundary, and regressions before editing implementation code.
- [x] Add a failing manager regression proving reversed challenge-scope order
      does not reopen the user agent during one authorization attempt.
- [x] Cover genuinely different scope sets, different authorization attempts,
      and denial-entry removal after a successful authorization.
- [x] Store a canonical scope set only in the internal denial key without
      changing public `OAuthScopes` equality, first-seen order, outbound
      parameters, or token fingerprints.
- [x] Update the OAuth protocol with semantic scope-set denial identity.
- [x] Run focused authorization-manager tests and all OAuth crate tests.
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
