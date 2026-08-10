# OAuth Issuer Preselection Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure every RFC 9728 authorization-server issuer is validated before
automatic or host selection.

**Architecture:** Move issuer-form and URL-policy validation into the protected
resource parser, the construction boundary for validated metadata. Reuse one
metadata-URL helper for early validation, selected-issuer fetch, and restart
discovery while preserving exact strings and order.

**Tech Stack:** Rust, Tokio, Unimock, RFC 9728/RFC 8414 metadata, Cargo
workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).
- Every advertised element must be an issuer accepted by the configured URL
  policy and RFC 8414 form, including no query or fragment.
- One invalid element fails the whole metadata response; do not filter.
- Preserve exact issuer strings and wire order for valid selection choices.
- Do not resolve DNS or fetch metadata for every candidate.

## Milestone 1: Validate The Entire Issuer Choice Set

At the end of this milestone, untrusted issuer strings cannot cross the host
selection boundary and the selected issuer keeps its existing pinned network
validation.

**Files:**

- Modify: `crates/ai-mcp-oauth/src/discovery/parsing.rs`
- Modify: `crates/ai-mcp-oauth/src/discovery/mod.rs`
- Modify: `crates/ai-mcp-oauth/src/selector.rs`
- Modify: `crates/ai-mcp-oauth/src/metadata.rs`
- Modify: `crates/ai-mcp-oauth/src/_tests_/discovery/validation_tests.rs`
- Modify: `docs/protocol/mcp-oauth.md`
- Modify: `crates/ai-mcp-oauth/README.md`

**Interfaces:**

- Extend private `parse_protected_resource` with the configured OAuth policy or
  config; no public API changes.
- Make private authorization-server metadata URL construction apply the
  configured `OAuthUrlPolicy` before RFC 8414 query rejection.
- Reuse `Error::InvalidUrl` and `Error::UnsafeUrl`; add no error variant.

- [x] Record the cycle-1 Codex finding, verify RFC 9728/RFC 8414 requirements,
      and have Claude Code Fable 5 validate severity, fail-closed behavior,
      helper ownership, regressions, and scope.
- [x] Add failure-first discovery tests proving a malformed, unsafe,
      query-bearing, fragment-bearing, or non-HTTPS issuer fails before the
      selector or authorization-server fetch.
- [x] Include a valid-first/invalid-second case that rejects implementations
      which validate only the selected issuer or silently filter bad entries.
- [x] Add a positive selector assertion proving valid issuer strings and wire
      order are unchanged.
- [x] Run the focused discovery tests and confirm the new ordering assertions
      fail for the expected pre-fix behavior.
- [x] Validate every issuer while constructing `ProtectedResourceMetadata`,
      use the configured development/production policy, and reuse the same
      helper for selected and restart discovery.
- [x] Preserve selector cancellation/membership checks, cache behavior,
      exact issuer comparison, loopback-development policy, and pinned
      transport dispatch.
- [x] Run focused discovery tests and the complete `ai-mcp-oauth` suite until
      all pass.
- [x] Align protocol, crate README, selector docs, and metadata field docs with
      preselection validation and fail-closed behavior.
- [x] Have Claude Code Fable 5 audit the completed implementation, ordering
      regressions, typed errors, documentation, and network boundaries.
- [x] Run the shared repository verification, commit/push, and post-push
      `cargo xtask review` tasks recorded in all three cycle-1 fix plans.
