# AI MCP OAuth

## Summary

Add `crates/ai-mcp-oauth` as the reusable, host-side OAuth companion to
`ai-mcp`. It will consume typed MCP authorization challenges, perform
OAuth 2.1 discovery and public-client authorization, manage refreshable tokens
through injected storage, and expose a non-interactive `JsonHttpAuth` adapter.

The approved source of truth is
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md). The
[`AI MCP crate`](ai-mcp-crate.md) plan must land its typed 401/403 challenge and
multi-value response-header contract before OAuth integration is completed.
Concrete browser UI, callback listeners, Keychain/database implementations,
and product connection policy remain host responsibilities.

## Milestone 1: Discovery And Public Client Registration

Create a usable OAuth protocol foundation with mockable side-effect boundaries.
At the end of this milestone, callers can safely turn an MCP authorization
challenge into validated resource/server metadata and a configured or
dynamically registered public client without browser interaction.

- [x] Re-verify the approved behavior against MCP authorization 2025-06-18,
      RFC 9728, RFC 8414, RFC 7591, RFC 7636, RFC 8707, and RFC 7009 before
      coding; record any standards discrepancy and request protocol direction
      before changing the approved contract.
- [x] Confirm that `ai-mcp` exports `McpAuthorizationChallenge`,
      `McpAuthorizationFailure`, distinct 401/403 errors, and all repeated
      `WWW-Authenticate` values; do not duplicate challenge parsing here.
- [x] Add `crates/ai-mcp-oauth` to workspace members and dependencies with
      workspace package metadata and internal `ai-mcp`/`json-http`
      dependencies.
- [x] Create the crate manifest and `test-support` feature; use `cargo add`
      without guessed versions for every external dependency needed for async
      traits, typed serde, URLs, HTTP, secrets, cryptographic randomness,
      SHA-256, and base64url. Add optional and dev Unimock dependencies using
      the established workspace pattern.
- [x] Keep `lib.rs` thin, add `#![warn(unreachable_pub)]`, use normal module
      resolution and narrow visibility, and split config, discovery,
      registration, transport, store, user-agent, manager, auth-hook, and error
      responsibilities before any Rust file approaches 300 lines.
- [x] Define fully typed public DTOs for canonical resource identity,
      protected-resource metadata, authorization-server metadata, configured
      and dynamic client registration, registration/credential keys, scopes,
      and unknown wire metadata.
- [x] Add `McpOAuthConfig` validation and documented defaults for HTTP and
      user-agent timeouts, state lifetime, response cap, validated redirects,
      metadata cache age, and refresh skew.
- [x] Define typed `Error`/`Result` contracts for unsafe or invalid URLs,
      resource/issuer mismatch, discovery status/schema failure, issuer
      selection cancellation, missing registration, DCR rejection, store
      failure, and internal failures without including secrets in diagnostics.
- [x] Define Unimock-enabled `OAuthHttpTransport`,
      `OAuthCredentialStore`, `AuthorizationServerSelector`, `OAuthClock`, and
      `OAuthRandom` traits and dyn aliases; use `Arc<dyn Trait + Send + Sync>`
      at every shared impure boundary.
- [x] Add failing tests for canonical URI normalization, fragment rejection,
      RFC 9728 well-known insertion for root and path resources, exact resource
      validation, unknown metadata, and bounded response handling.
- [x] Implement protected-resource discovery from a challenge URL with
      deterministic same-resource fallback when it is absent, plus cache
      invalidation when a new challenge advertises changed metadata.
- [x] Add failing tests for one/multiple issuer selection, exact RFC 8414
      issuer validation, missing endpoints, unsafe endpoints, redirect
      validation, and metadata cache expiry before implementing authorization
      server discovery.
- [x] Add failing tests for registration precedence, cache keys, public-client
      DCR request/response mapping, conditional refresh-grant registration,
      unsupported public-client token authentication, ignored returned client
      secrets, unsupported registration, and redacted registration diagnostics
      before implementing configured/cached/DCR resolution.
- [x] Implement a production HTTP transport that disables automatic redirects,
      validates every redirect target, enforces URL policy after DNS
      resolution, pins or verifies the connected peer against validated
      addresses, and caps time, redirect count, and response bytes.
- [x] Add the crate README with Responsibilities, What This Crate Does, Quick
      Start, Development, Key Code, and Related Docs sections, documenting
      discovery and registration without suggesting that the crate owns UI or
      persistent storage.
- [x] Run `cargo fmt --all -- --check`, targeted Clippy with warnings denied,
      `cargo test -p ai-mcp-oauth --all-features`,
      `cargo xtask rust-file-length-lint --all`, and `cargo xtask check`; fix
      all failures until the milestone is green.
- [x] Run `git add -A`, commit the green discovery/registration milestone with
      a descriptive Conventional Commit whose title is at most 50 characters,
      and push the current branch.
- [x] After the push, run `cargo xtask review`; do not apply findings
      automatically, and report them with severity, context, impact, lettered
      options, and a recommended option for user decision.

## Milestone 2: PKCE Authorization And Token Lifecycle

Add interactive authorization through injected host capabilities and a
concurrency-safe token lifecycle. At the end of this milestone, a host can
authorize explicitly, persist tokens, inject them into MCP requests, refresh
them without user interaction, and disconnect.

- [x] Define Unimock-enabled `OAuthUserAgent` and `McpOAuthManager` traits,
      their dyn aliases, and the exact authorization context, browser request,
      callback response, token-set, and connection-summary DTOs; keep explicit
      authorize, forced refresh, and disconnect operations distinct.
- [x] Add failing deterministic tests for RFC 7636 verifier/challenge vectors,
      verifier length, cryptographically random state, state expiry,
      single-use enforcement, mismatches, callback OAuth errors, cancellation,
      and timeout before implementing PKCE/state handling.
- [x] Build authorization URLs only from validated metadata, using S256,
      exact registered redirect URI, canonical `resource`, and the minimum
      host-approved plus challenge-requested scopes.
- [x] Implement `OAuthUserAgent` orchestration without a concrete browser:
      pass one validated URL to the host and accept only a typed callback
      result; never execute shell commands or process arbitrary callback URLs.
- [x] Add failing tests that require `resource` in both authorization and token
      requests, the original verifier/redirect URI during code exchange,
      Bearer token type, absent-expiry behavior, requested/granted/refresh scope
      fallback, and secret redaction.
- [x] Implement one-time authorization-code exchange and atomically persist the
      resulting token set under user/account, resource, issuer, client ID,
      and redirect URI; retain the granted scopes inside the token set.
- [x] Add failing clock/store tests for the 60-second default refresh skew,
      configurable skew, unrelated-key concurrency, same-key single-flight,
      refresh-token rotation, omitted replacement refresh tokens,
      `invalid_grant`, transient failures, and atomic storage failure.
- [x] Implement non-interactive refresh so rotated tokens replace old tokens
      before use, explicit refresh reports `InteractionRequired` when no usable
      refresh remains, auth-hook `invalid_grant` clears credentials and omits
      the header for an authoritative MCP challenge, and transient errors
      preserve stored credentials.
- [x] Implement `RefreshingMcpAuth` as a resource-bound `JsonHttpAuth`: inject
      fresh tokens, refresh when possible, leave headers unchanged when no
      credential exists, never invoke the user agent, and never send a token
      under a different resource identity.
- [x] Implement disconnect with best-effort revocation only through a
      validated advertised endpoint, followed by unconditional local deletion;
      keep network failure distinct from local deletion failure, retain cached
      registration by default, and support explicit local registration removal
      without RFC 7592 remote management.
- [x] Update the README with explicit authorize, hook construction, refresh,
      incremental-scope, and disconnect examples using fake/in-memory host
      implementations only.
- [x] Run formatting, targeted Clippy with warnings denied, every crate feature
      combination, targeted tests, Rust file-length lint, and
      `cargo xtask check`; fix failures until the milestone is green.
- [x] Run `git add -A`, commit the green authorization/token milestone with a
      descriptive Conventional Commit whose title is at most 50 characters,
      and push the current branch.
- [x] After the push, run `cargo xtask review`; do not apply findings
      automatically, and report them with severity, context, impact, lettered
      options, and a recommended option for user decision.

## Milestone 3: MCP Integration And Security Hardening

Exercise the complete boundary against realistic local servers and finish
documentation and security validation. At the end of this milestone, a host can
connect, refresh, elevate scope, and disconnect through `ai-mcp` without real
credentials or unsafe discovery behavior.

- [x] Add in-process fake protected-resource and authorization servers covering
      resource metadata, RFC 8414 metadata, DCR, authorization callback, token,
      refresh, rotation, revocation, MCP 401/403, and authenticated MCP success.
- [x] Add an end-to-end initial connection test: unauthenticated initialize →
      typed challenge → discovery → DCR → injected user-agent callback → token
      storage → one host-owned initialize retry.
- [x] Add stored-token reuse and concurrent refresh tests proving that every
      MCP POST, SSE side-response POST, and DELETE receives the same
      resource-bound Bearer token.
- [x] Add `invalid_grant` and post-refresh 401 tests proving credentials are
      cleared safely and the host receives one actionable interaction outcome,
      with no authorization or request retry loop.
- [x] Add 403 `InsufficientScope` tests for explicit incremental consent,
      minimum requested scopes, granted subsets, user denial, and
      denied-scope prompt-loop suppression; other 403 responses stay denied.
- [x] Add adversarial tests for dangerous schemes, user info/fragments,
      private/reserved/link-local/metadata IPs, alternate IP encodings, DNS
      rebinding, unsafe ports, malicious authorization URLs, and redirect
      chains. Permit loopback HTTP only through explicit development policy.
- [x] Add log/error/debug capture tests proving access tokens, refresh tokens,
      codes, verifiers, state, and any configured credentials are always
      redacted.
- [x] Add a credential-free workspace smoke test using only in-process servers;
      add an ignored environment-gated live test only if a stable OAuth-enabled
      MCP test server becomes available.
- [x] Update the root README interface/key-code maps and both MCP crate READMEs;
      reconcile this protocol, the `ai-mcp` protocol, and their plans with the
      final public behavior.
- [x] Audit all changed Rust for public/module documentation, trait-backed
      impure behavior, typed errors, import order, test placement, forbidden
      panic/unwrap/expect/map_err use, secret leakage, and the 300-line cap.
- [x] Run `cargo fmt --all -- --check`,
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      `cargo test --workspace --all-features`,
      `cargo xtask rust-file-length-lint --all`, credential-free smoke tests,
      and `cargo xtask check`; keep working until the test pass rate is 100%.
- [x] Mark every completed TODO and milestone in this plan, move its
      `plans/README.md` link from Active to Completed, and review
      `git diff origin/main...` plus `git status` for omissions or unrelated
      changes.
- [x] Run `git add -A`, commit all completed integration work with a
      descriptive Conventional Commit whose title is at most 50 characters,
      and push the current branch.
- [x] After the push, run `cargo xtask review` against `origin/main`; do not
      change findings automatically, and report every item with a number,
      severity, context, impact of doing nothing, lettered solution options,
      and a recommended option.
