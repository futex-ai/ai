# OAuth Callback No-State Constructors Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let host OAuth user agents construct successful and error callback
responses whose `state` parameter was omitted without type annotations or
direct secret-field construction.

**Architecture:** Preserve the existing generic constructors for callbacks
that contain state so current `Some(&str)`, `Some(String)`, and
`Option<&String>` callers remain source-compatible. Add explicit constructors
for the omitted-state cases that wrap the authorization code and set the
variant state to `None`.

**Tech Stack:** Rust, secrecy, Cargo workspace checks.

## Global Constraints

- Preserve `OAuthAuthorizationResponse` and both existing constructors.
- Keep callback codes and states wrapped in `SecretString`.
- Keep every callback response `Debug` representation secret-safe.
- Treat omitted state as representable input, not as a valid state match; the
  manager must continue to reject missing or mismatched state.
- This work resolves the finding from the tenth and final review cycle; the
  codex-review cycle limit does not permit an eleventh review run.

## Milestone 1: Add Omitted-State Callback Helpers

At the end of this milestone, hosts can safely create either callback variant
without state through an inference-free public helper, while existing
present-state integrations compile unchanged.

**Files:**

- Modify: `crates/ai-mcp-oauth/src/user_agent.rs`
- Modify: `crates/ai-mcp-oauth/src/_tests_/redaction_tests.rs`
- Modify: `crates/ai-mcp-oauth/README.md`
- Modify: `docs/protocol/mcp-oauth.md`
- Modify: `plans/README.md`

**Interfaces:**

- Add `OAuthAuthorizationResponse::authorized_without_state`.
- Add `OAuthAuthorizationResponse::oauth_error_without_state`.
- Preserve `OAuthAuthorizationResponse::authorized` and
  `OAuthAuthorizationResponse::oauth_error`.

- [x] Reproduce the cycle-10 compiler error for literal `None` independently
      against the built public crate and audit all existing constructor call
      shapes.
- [x] Have Claude Code Fable 5 validate the finding, P3 severity, API options,
      non-breaking constructor design, and regression plan before production
      edits.
- [x] Add failure-first tests that call both no-state helpers without type
      annotations, inspect the resulting variants, round-trip the wrapped
      authorization code, and cover their redacted `Debug` output.
- [x] Confirm the regression test fails to compile before the production API
      is added.
- [x] Add the two documented no-state constructors without changing existing
      constructor signatures or state-validation behavior.
- [x] Document present-state and omitted-state callback construction in the
      OAuth protocol and crate README.
- [x] Run formatting and the complete `ai-mcp-oauth` test suite.
- [x] Have Claude Code Fable 5 audit the implementation, public API,
      regressions, documentation, compatibility, and redaction behavior.
- [x] Run workspace formatting, Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit the completed fix and plan bookkeeping with a
      Conventional Commit title no longer than 50 characters, and push the
      current branch.
- [x] Record that the finding arose in review cycle 10 and no cycle 11 is
      permitted by the codex-review skill.
