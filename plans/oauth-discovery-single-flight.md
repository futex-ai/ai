# OAuth Discovery Single-Flight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` while implementing this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent concurrent OAuth discovery for one MCP resource from showing
duplicate authorization-server selection prompts or racing cache state.

**Architecture:** Add a private per-canonical-resource mutex registry to the
default discovery service. Acquire that resource lock before reading the clock
or checking the resource-keyed cache, and hold it through uncached discovery
and any conditional cache insert. Cacheable successes coalesce through the
normal cache. Non-cacheable results, cancellation, and errors remain
caller-specific and run sequentially. Distinct resources use distinct locks.

**Tech Stack:** Rust, Tokio synchronization, Unimock, OAuth metadata cache
control, Cargo workspace checks.

## Global Constraints

- The protocol source of truth is
  [`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).
- Lock by canonical resource, matching the cache key; do not key by metadata
  URL or serialize unrelated resources globally.
- Never hold the lock-registry mutex across discovery, selection, or network
  awaits.
- Read the clock and perform the only cache lookup after acquiring the
  resource lock so waiters cannot use stale time.
- Do not cache or share `no-store`, `no-cache`, malformed-`max-age`, failed, or
  cancelled discovery results.
- Preserve typed errors, selector membership validation, URL validation,
  metadata invalidation, and the public API.

## Milestone 1: Serialize Discovery Per Resource

At the end of this milestone, identical concurrent cacheable discovery invokes
the selector once, non-cacheable/error outcomes remain independent, and
different resources can discover concurrently.

**Files:**

- Modify: `crates/ai-mcp-oauth/src/discovery/mod.rs`
- Modify: `crates/ai-mcp-oauth/src/_tests_/discovery/mod.rs`
- Add: `crates/ai-mcp-oauth/src/_tests_/discovery/concurrency_tests.rs`
- Add: `crates/ai-mcp-oauth/src/_tests_/discovery/concurrency_support.rs`
- Add: `crates/ai-mcp-oauth/src/_tests_/discovery/cancellation_tests.rs`
- Modify: `docs/protocol/mcp-oauth.md`
- Modify: `crates/ai-mcp-oauth/README.md`

**Interfaces:**

- Preserve `McpOAuthDiscovery::discover` and `authorization_server`.
- Add only private resource-lock state and a private lock lookup helper to
  `DefaultMcpOAuthDiscovery`.
- Use `Arc<Mutex<()>>` values under a short-lived registry mutex.

- [x] Record the cycle-2 Codex finding and independently reproduce the
      cache-miss/selector/cache-insert race.
- [x] Have Claude Code Fable 5 validate severity, resource-only keying,
      post-lock time/cache ordering, cache-control semantics, error behavior,
      deterministic regressions, and scope before editing implementation.
- [x] Add a failure-first gated regression proving two identical concurrent
      cacheable discoveries invoke the selector and metadata fetch only once
      and return the same result.
- [x] Add concurrency controls proving unrelated resources do not block and
      `no-store` discovery serializes but executes separately.
- [x] Add an error control proving a failed leader is not shared or cached and
      a waiting caller retries independently.
- [x] Add the Fable-recommended cancellation hardening regression proving an
      aborted queued waiter cannot retain the resource lock or block a later
      independent non-cacheable discovery.
- [x] Run the focused concurrency tests and confirm the duplicate-selector
      regression fails for the expected pre-fix behavior.
- [x] Add the resource-lock registry and acquire the resource guard before the
      clock read and sole cache check.
- [x] Keep cache removal, uncached discovery, and conditional insert within
      the resource guard while leaving `authorization_server` unchanged.
- [x] Run focused discovery tests and the complete `ai-mcp-oauth` suite until
      all pass.
- [x] Align the protocol and crate README with per-resource serialization,
      cacheable-success coalescing, and caller-specific non-cacheable/errors.
- [x] Have Claude Code Fable 5 audit the completed concurrency implementation,
      forced interleavings, cache-control/error behavior, documentation, and
      unrelated-resource independence.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, file-size violations, and untracked files.
- [x] Run `git add -A`, commit all completed cycle-2 fixes with a Conventional
      Commit title no longer than 50 characters, and push the current branch.
- [x] After the push, run the next `cargo xtask review` cycle against
      `origin/main`; investigate every finding and repeat within the
      invocation's ten-cycle limit.
