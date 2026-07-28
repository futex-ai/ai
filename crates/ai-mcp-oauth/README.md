# ai-mcp-oauth

`ai-mcp-oauth` is the host-side OAuth companion to `ai-mcp`. Depend on it when
an application needs to turn typed MCP Bearer challenges into validated OAuth
metadata, a public client registration, and resource-bound credentials while
retaining control of browser UX and secure persistence.

## Responsibilities

- Discover RFC 9728 protected-resource and RFC 8414 authorization-server
  metadata.
- Resolve configured or cached public clients and perform RFC 7591 dynamic
  registration.
- Enforce HTTPS, destination, redirect, timeout, response-size, and metadata
  cache bounds.
- Provide trait-backed seams for HTTP, DNS, time, randomness, issuer selection,
  credential storage, and user interaction.
- Run PKCE authorization, token refresh, request authentication, and local
  disconnect without exposing credential material.

## What This Crate Does

`DefaultMcpOAuthDiscovery` consumes the `McpAuthorizationChallenge` returned by
`ai-mcp`, validates exact resource identity and every advertised issuer before
selection, and selects an authorization server through a host-provided
`AuthorizationServerSelector`. One malformed or unsafe issuer fails discovery
instead of exposing a partially validated choice set.
`DefaultOAuthClientRegistry` then uses a configured registration, a host store,
or public-client dynamic registration in that order.
Configured and cached registrations must match the approved redirect URI and
client name and carry a non-empty client ID; mismatches fail without issuing a
new dynamic registration.
Before registry or browser work, interactive authorization rejects a non-empty
`grant_types_supported` list that excludes `authorization_code`; omitted grant
metadata retains the RFC 8414 authorization-code default.
It also rejects existing client-owned authorization query parameters before
registration while retaining non-reserved endpoint extension parameters.
Only authorization-required, invalid-token, and insufficient-scope challenges
can start authorization. Forbidden and invalid-request challenges return
distinct typed errors before discovery or any other side effect because a new
grant cannot repair those outcomes.

The production transport disables automatic redirects, follows only validated
metadata GET redirects, and never redirects a registration, token, or
revocation POST. Each HTTP-hop timeout covers DNS, connection, headers, and
streamed response bytes; an unrepresentable deadline is rejected as invalid
configuration. Validated addresses are pinned before dispatch. Environment and
system proxies are ignored so they cannot bypass those pins; proxy-only egress
therefore fails closed as a transport error. Valid JSON is retained at every
status. Non-JSON error bodies preserve their status with a `null` body,
successful discovery, registration, and token responses require JSON, and RFC
7009 revocation success is determined only by status.
Before browser handoff, the manager separately resolves the initial
authorization hostname within the same HTTP timeout and requires every address
to satisfy the same policy. A stalled lookup surfaces as a typed DNS failure.
This preflight cannot pin an external browser's later DNS lookup or redirects;
the host and user-agent implementation retain that responsibility.
HTTP loopback is available only through the explicit development policy: local
hostnames and literals must resolve exclusively to loopback addresses, blocked
ports remain blocked, and deprecated IPv4 6to4 relay anycast, IPv4-compatible,
both NAT64 prefixes, IPv6 discard/dummy, IETF protocol-assignment,
documentation, IPv6 6to4, SRv6 SID, and deprecated site-local destinations are
rejected. Metadata marked `no-store` or `no-cache`, or carrying an invalid
`max-age`, is never reused. A cached multi-issuer discovery result retains its
host-selected issuer for that cache lifetime. The crate does not provide a
browser, callback listener, Keychain/database implementation, or product UI.

## Quick Start

Canonicalize the same endpoint string that is passed to `ai-mcp`:

```rust
use ai_mcp_oauth::{CanonicalMcpResource, OAuthUrlPolicy};

let resource = CanonicalMcpResource::parse(
    "https://tools.example.com/mcp",
    &OAuthUrlPolicy::default(),
)?;
assert_eq!(
    resource.protected_resource_metadata_url()?,
    "https://tools.example.com/.well-known/oauth-protected-resource/mcp"
);
# Ok::<(), ai_mcp_oauth::Error>(())
```

Construct discovery and registration services with host-owned trait objects:

```rust,no_run
use std::sync::Arc;

use ai_mcp_oauth::{
    DefaultMcpOAuthDiscovery, DefaultOAuthClientRegistry, McpOAuthConfig,
    ReqwestOAuthHttpTransport, SystemOAuthClock,
};

# fn build(
#   selector: ai_mcp_oauth::DynAuthorizationServerSelector,
#   store: ai_mcp_oauth::DynOAuthCredentialStore,
# ) -> ai_mcp_oauth::Result<()> {
let config = McpOAuthConfig::default();
let transport = Arc::new(ReqwestOAuthHttpTransport::new());
let discovery = DefaultMcpOAuthDiscovery::new(
    transport.clone(),
    selector,
    Arc::new(SystemOAuthClock),
    config.clone(),
)?;
let registry = DefaultOAuthClientRegistry::new(transport, store, config)?;
# let _ = (discovery, registry);
# Ok(())
# }
```

The host calls discovery after `ai-mcp` returns a typed 401/403 challenge,
shows the already validated issuer choices when selection is needed, and
supplies secure registration storage. It must not silently select among
multiple issuers. Call
`McpOAuthManager::authorize` only for authorization-required, invalid-token, or
insufficient-scope challenges; `InvalidRequest` and `Forbidden` are rejected
without opening a browser.

Authorize only from an explicit host action, then bind the stored credential to
the same canonical resource used by the MCP client:

```rust,no_run
use std::sync::Arc;

use ai_mcp::McpAuthorizationChallenge;
use ai_mcp_oauth::{
    DynMcpOAuthManager, DynOAuthRequestTokenProvider, OAuthAuthorizationContext,
    RefreshingMcpAuth,
};
use json_http::{DynJsonHttpAuth, JsonHttpAuth};

async fn authorize_and_build_hook(
    manager: DynMcpOAuthManager,
    token_provider: DynOAuthRequestTokenProvider,
    challenge: McpAuthorizationChallenge,
    context: OAuthAuthorizationContext,
) -> ai_mcp_oauth::Result<DynJsonHttpAuth> {
    let connection = manager.authorize(&challenge, &context).await?;
    let auth = RefreshingMcpAuth::new(
        context.resource.clone(),
        connection.key,
        token_provider,
    )?;
    Ok(Arc::new(auth) as Arc<dyn JsonHttpAuth>)
}
```

`RefreshingMcpAuth` performs only non-interactive loads and refreshes. If no
usable token exists, it leaves the request unauthenticated so `ai-mcp` can
return the authoritative challenge. The host owns the single retry after a
successful authorization or refresh. A token without a refresh token remains
usable through its actual expiry, including inside the configured refresh
skew; a known-expired token is never sent. An empty refresh token in a token
response is treated as absent, retaining the prior refresh token during
rotation when one exists. Token endpoint rejections retain their HTTP status
and typed OAuth error. In particular, a rejected authorization code does not
mutate stored credentials, while only an RFC-compliant HTTP 400
`invalid_grant` triggers stale-credential cleanup in the refresh path.

Forced refresh, incremental consent, and disconnect remain separate,
host-controlled operations:

```rust,no_run
use ai_mcp::{McpAuthorizationChallenge, McpAuthorizationFailure};
use ai_mcp_oauth::{
    DynMcpOAuthManager, OAuthAuthorizationContext, OAuthCredentialKey,
};

async fn maintain_connection(
    manager: DynMcpOAuthManager,
    key: OAuthCredentialKey,
    context: OAuthAuthorizationContext,
) -> ai_mcp_oauth::Result<()> {
    manager.refresh(&key).await?;

    let incremental = McpAuthorizationChallenge {
        failure: McpAuthorizationFailure::InsufficientScope,
        resource_metadata_url: None,
        scopes: vec!["tools.write".to_owned()],
        error_description: None,
        raw_www_authenticate: Vec::new(),
    };
    manager.authorize(&incremental, &context).await?;

    manager.disconnect(&key).await
}
```

Examples and tests use only injected fake or in-memory stores, user agents, and
servers. Applications must provide secure persistence and platform browser/
callback handling. Disconnect retains the cached client registration; a host
that wants to forget it can explicitly call
`OAuthCredentialStore::delete_registration` with
`OAuthCredentialKey::registration_key`. No RFC 7592 remote deletion occurs.
Disconnect serializes with refresh for the same credential, so a completed
refresh cannot restore tokens after local removal. Interactive authorization
uses the same lock only for its final token write, so the newly approved grant
wins over an older in-flight refresh without blocking browser interaction. A
grant that finishes after an already-completed disconnect is treated as a new
connection. If loading stored tokens fails, disconnect skips remote revocation
but still attempts key-based local deletion. Successful deletion preserves the
typed load error; if deletion also fails, `LocalTokenDeletionFailed` takes
precedence with `revocation_failed = true`. Any 2xx revocation status counts as
success regardless of an empty, JSON, or non-JSON response body.

## Development

```sh
cargo test -p ai-mcp-oauth --all-features
cargo test -p ai-mcp-oauth --test oauth_integration
cargo clippy -p ai-mcp-oauth --all-targets --all-features -- -D warnings
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
```

Unit tests use injected Unimock boundaries. The integration suite runs real
MCP and OAuth reqwest transports against a credential-free loopback server,
including DCR, PKCE callback, refresh, revocation, 401/403, SSE side responses,
and DELETE authentication. Revocation coverage includes a successful
plain-text response body.

### Key Code

- `src/discovery/` — protected-resource and authorization-server discovery
- `src/registration.rs` — configured, cached, and dynamic client resolution
- `src/manager/` — explicit authorization, refresh, and disconnect orchestration
- `src/auth_hook.rs` — resource-bound non-interactive MCP authentication
- `src/pkce.rs` and `src/state.rs` — S256 and one-time callback-state handling
- `src/transport/` — bounded HTTP seam and DNS-pinned reqwest implementation
- `src/url_policy.rs` — endpoint syntax and resolved-address policy
- `src/resource.rs` — canonical resource identity and ordered scopes
- `src/store.rs` — host-controlled secure persistence boundary
- `src/error.rs` — typed, secret-safe public errors
- `tests/oauth_integration.rs` — complete credential-free OAuth/MCP flow
- `tests/support/` — in-memory host boundaries and loopback protocol server

### Related Docs

- [`../../docs/protocol/mcp-oauth.md`](../../docs/protocol/mcp-oauth.md)
- [`../../docs/protocol/ai-mcp.md`](../../docs/protocol/ai-mcp.md)
- [`../../plans/ai-mcp-oauth.md`](../../plans/ai-mcp-oauth.md)
- [`../ai-mcp/README.md`](../ai-mcp/README.md)
