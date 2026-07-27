# OAuth Grant-Type Metadata Validation

## Summary

Fail before registration or browser side effects when authorization-server
metadata explicitly advertises grant types that exclude
`authorization_code`. Preserve the RFC 8414 default when the optional
`grant_types_supported` metadata is omitted.

The source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md). The validation
belongs to interactive authorization because refresh consumes the same server
metadata without requiring authorization-code support.

## Milestone 1: Validate Authorization-Code Support

At the end of this milestone, interactive authorization accepts omitted grant
metadata, rejects an explicitly incompatible non-empty list with a typed error,
and performs no registration or browser work after that rejection.

- [x] Record the Codex P2 finding and independently confirm its validity,
      calibrated P3 severity, RFC-default nuance, and solution with Claude Code
      Fable 5 before editing implementation code.
- [x] Add a failing manager regression proving an issuer that advertises only
      incompatible grants is rejected before registration and browser side
      effects.
- [x] Add coverage proving omitted grant metadata retains the RFC 8414
      authorization-code default.
- [x] Add a typed unsupported-authorization-code error, a metadata capability
      helper, and a pre-registration guard in interactive authorization.
- [x] Update the OAuth protocol and crate README with the explicit-list
      rejection, omitted-list default, and typed failure.
- [x] Run focused authorization-manager tests and all affected crate tests.
- [x] Have Claude Code Fable 5 audit the completed implementation, tests, and
      documentation and resolve every valid solution issue.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete implementation diff for omissions, unrelated
      changes, Markdown errors, and untracked files.
- [x] Run `git add -A`, commit the checked work with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run `cargo xtask review` against `origin/main` and
      investigate every finding under the active review workflow.
- [ ] Once review has no valid findings, mark this plan completed in
      `plans/README.md`, commit and push the plan bookkeeping, and run the
      final post-push review.
