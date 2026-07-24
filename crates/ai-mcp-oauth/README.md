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

## Development

```sh
cargo test -p ai-mcp-oauth --all-features
cargo clippy -p ai-mcp-oauth --all-targets --all-features -- -D warnings
cargo xtask rust-file-length-lint --all
```

Tests use injected Unimock boundaries and credential-free loopback servers.

### Key Code

- `src/discovery/` — protected-resource and authorization-server discovery
- `src/registration.rs` — configured, cached, and dynamic client resolution
- `src/transport/` — bounded HTTP seam and DNS-pinned reqwest implementation
- `src/url_policy.rs` — endpoint syntax and resolved-address policy
- `src/resource.rs` — canonical resource identity and ordered scopes
- `src/store.rs` — host-controlled secure persistence boundary
- `src/error.rs` — typed, secret-safe public errors

### Related Docs

- [`../../docs/protocol/mcp-oauth.md`](../../docs/protocol/mcp-oauth.md)
- [`../../docs/protocol/ai-mcp.md`](../../docs/protocol/ai-mcp.md)
- [`../../plans/ai-mcp-oauth.md`](../../plans/ai-mcp-oauth.md)
- [`../ai-mcp/README.md`](../ai-mcp/README.md)
