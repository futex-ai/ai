# OAuth IPv4 Special-Range Classification Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reject only the non-global IPv4 special-purpose assignments within
`192.0.0.0/16` while allowing ordinary public OAuth destinations.

**Architecture:** Narrow the current broad `/16` rejection to the IANA
`192.0.0.0/24` protocol-assignment block, retaining its globally reachable
`.9` and `.10` exceptions. Keep the separate TEST-NET-1 and all other private,
reserved, transition, benchmark, documentation, and mapped-address rules.

**Tech Stack:** Rust, IANA address registry, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).
- Preserve all public OAuth policy and error APIs.
- Apply identical classification to literals, DNS results, and IPv4-mapped
  IPv6 addresses.
- Keep `192.0.2.0/24` and every existing non-global range blocked.
- Do not replace or broadly relax the full destination-security classifier.

## Milestone 1: Narrow the IETF Assignment Boundary

At the end of this milestone, globally routable addresses elsewhere in
`192.0.0.0/16` pass production HTTPS policy while exact non-global IANA
assignments remain blocked.

**Files:**

- Modify: `crates/ai-mcp-oauth/src/url_policy.rs`
- Modify: `crates/ai-mcp-oauth/src/_tests_/url_policy_adversarial_tests.rs`
- Modify: `docs/protocol/mcp-oauth.md`
- Modify: `crates/ai-mcp-oauth/README.md`
- Modify: `plans/README.md`

**Interfaces:**

- Preserve `OAuthUrlPolicy` and its existing typed rejection behavior.
- Change only the private IPv4 address classifier.

- [x] Record the cycle-7 Codex finding and independently trace literal, DNS,
      redirect, authorization-preflight, and IPv4-mapped policy paths.
- [x] Verify the affected blocks and more-specific exceptions against the
      current IANA IPv4 Special-Purpose Address Registry.
- [x] Have Claude Code Fable 5 validate the finding, severity, exact IANA
      predicate, security boundary, regression matrix, and documentation
      scope before editing production code.
- [x] Add failure-first literal and DNS-classifier regressions for public
      `192.0.1.1`, `192.0.3.1`, and the end of the affected `/16`.
- [x] Add boundary coverage that keeps representative `192.0.0.0/24` values
      blocked while allowing globally reachable `.9` and `.10`.
- [x] Add IPv4-mapped IPv6 coverage for newly allowed and retained blocked
      assignments.
- [x] Confirm the newly public-address regressions fail before the fix.
- [x] Narrow the protocol-assignment predicate without changing any other
      special-purpose range.
- [x] Run focused URL-policy tests and the complete `ai-mcp-oauth` suite.
- [x] Align protocol and crate README address-policy semantics.
- [x] Have Claude Code Fable 5 audit the implementation, regressions,
      documentation, and preserved SSRF protections.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit the completed fix with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [ ] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding within the invocation's
      ten-cycle limit.
