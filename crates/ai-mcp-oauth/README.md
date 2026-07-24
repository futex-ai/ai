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
`ai-mcp`, validates exact resource and issuer identity, and selects an
authorization server through a host-provided `AuthorizationServerSelector`.
`DefaultOAuthClientRegistry` then uses a configured registration, a host store,
or public-client dynamic registration in that order.

The production transport disables automatic redirects, validates each hop
before dispatch, checks every DNS result, pins validated addresses, and bounds
time and bytes. HTTP loopback is available only through the explicit
development policy. The crate does not provide a browser, callback listener,
Keychain/database implementation, or product UI.

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
shows issuer selection when needed, and supplies secure registration storage.
It must not silently select among multiple issuers.

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
successful authorization or refresh.

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
and DELETE authentication.

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
