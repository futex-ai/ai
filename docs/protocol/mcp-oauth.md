# Spec: `ai-mcp-oauth` — host-side MCP OAuth

Status: approved by Cal on 24 Jul 2026. This is the durable source of truth for
[`plans/ai-mcp-oauth.md`](../../plans/ai-mcp-oauth.md).

## Purpose

Add a reusable `crates/ai-mcp-oauth` library that turns the typed HTTP 401 and
403 challenges emitted by `ai-mcp` into OAuth 2.1 authorization for remote MCP
servers. It owns discovery, public-client registration, PKCE, token exchange,
refresh, and the `JsonHttpAuth` adapter. The embedding host owns browser UX,
authorization-server selection, secure persistence, and user consent through
injected traits.

## Responsibilities

- Discover protected-resource and authorization-server metadata.
- Register or resolve a public OAuth client.
- Run authorization-code flow with PKCE through an injected user agent.
- Persist, refresh, rotate, and remove tokens through an injected secure store.
- Inject a valid Bearer token into every MCP HTTP request.
- Surface typed interaction, scope, discovery, and token failures to the host.
- Enforce URL, audience, redirect, state, scope, and secret-handling controls.

## Non-goals

- OAuth inside `ai-mcp`; that crate only parses challenges and applies auth.
- Concrete browser, callback listener, Keychain, database, or product UI.
- OAuth for stdio MCP servers.
- Password, implicit, device-code, client-credentials, or token-exchange grants.
- Confidential-client secrets, DPoP, OIDC login semantics, or token
  introspection in v1.
- Remote dynamic-client management or deletion through RFC 7592.
- Automatically granting scopes or silently choosing among multiple issuers.
- Passing an MCP access token through to a downstream API.

## Standards target

The implementation targets MCP authorization revision **2025-06-18** and:

- <https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization>
- <https://datatracker.ietf.org/doc/html/rfc9728>
- <https://datatracker.ietf.org/doc/html/rfc8414>
- <https://datatracker.ietf.org/doc/html/rfc7591>
- <https://datatracker.ietf.org/doc/html/rfc7636>
- <https://datatracker.ietf.org/doc/html/rfc8707>
- <https://datatracker.ietf.org/doc/html/rfc7009>

## End-to-end flow

1. The host constructs a `RefreshingMcpAuth` for one canonical MCP resource
   URI and injects it into `StreamableHttpMcpClient`.
2. The hook injects a fresh stored token, refreshes one non-interactively, or
   sends no Authorization header when no usable credential exists.
3. A protected server returns `Error::AuthorizationRequired` (401) or
   `Error::Forbidden` (403) with `McpAuthorizationChallenge`.
4. On a 401 with an existing credential, the host may call
   `McpOAuthManager::refresh` once. A new connection, unusable refresh, or
   explicit insufficient-scope consent uses `McpOAuthManager::authorize`.
   Interactive work never runs from `JsonHttpAuth::apply_headers`.
5. The manager stores the resulting token set atomically.
6. The host retries the interrupted MCP operation once. Further 401/403
   responses are surfaced; no unbounded authorization or retry loop is allowed.
7. Later requests refresh within the configured expiry skew under a
   per-credential single-flight lock.

For a 403 with `InsufficientScope`, authorization is incremental and requires
explicit host consent. Other 403 responses remain denied and do not
automatically open a browser.

## Configuration defaults

`McpOAuthConfig` defaults to a 30-second HTTP timeout, 10-minute user-agent
timeout/state lifetime, 1 MiB response cap, three validated redirects,
one-hour maximum metadata cache age, and 60-second refresh skew. Every value is
configurable, positive, and bounded before network or authorization work.

## Resource identity and discovery

- The credential resource is the most specific absolute MCP endpoint URI.
  Scheme and host are lowercase, fragments are rejected, and trailing-slash
  handling is stable. Use the identical canonical string for discovery
  validation, OAuth `resource`, storage keys, and retries.
- Prefer the challenge's `resource_metadata_url` after applying the OAuth URL
  policy. When absent, derive the RFC 9728
  `/.well-known/oauth-protected-resource` URL by inserting the well-known
  segment between the authority and resource path.
- The protected-resource response must be JSON, stay within configured size
  and timeout limits, and return `resource` exactly equal to the canonical MCP
  resource. Unknown fields are ignored.
- `authorization_servers` must contain at least one valid issuer. One issuer
  may be selected automatically. Multiple issuers require the injected
  `AuthorizationServerSelector`; cancellation is a typed outcome.
- Fetch RFC 8414 metadata for the selected issuer. The returned `issuer` must
  exactly match, and authorization, token, and optional registration endpoints
  must pass the same URL policy.
- A fresh `WWW-Authenticate` resource-metadata URL invalidates cached discovery
  for that resource. Otherwise, metadata caching follows HTTP cache directives
  and is bounded by a configurable maximum age.

## Client registration

Resolve a public client in this order:

1. A configured registration supplied by the host for the issuer.
2. A previously stored dynamic registration for the same issuer, redirect URI,
   and client metadata.
3. RFC 7591 Dynamic Client Registration when `registration_endpoint` exists.
4. `Error::ClientRegistrationRequired` with issuer and redirect details.

Dynamic registration sends the exact host-approved redirect URI,
`response_types = ["code"]`, and `token_endpoint_auth_method = "none"`.
`grant_types` always contains `authorization_code` and adds `refresh_token`
only when server metadata advertises it. Reject an issuer that does not support
public-client token authentication. Persist the returned client ID before
authorization. V1 ignores a returned client secret and never treats an
embedded desktop/mobile secret as confidential.

## Authorization and token exchange

- Use an external user agent through `OAuthUserAgent`; never embed credentials
  or launch a URL through a shell.
- Redirect URIs must be HTTPS or explicit loopback HTTP and must exactly match
  the registered value.
- Generate state from 32 random bytes, encode it base64url without padding,
  and make it single-use for the configured 10-minute default lifetime. Reject
  missing, expired, reused, or mismatched state.
- Generate the RFC 7636 verifier from a separate 32 random bytes encoded
  base64url without padding and derive its `S256` challenge. `plain` is
  unsupported.
- Include the canonical `resource` in both authorization and token requests.
- Request only host-approved baseline scopes plus scopes from the triggering
  challenge. Never request the entire advertised scope catalog by default.
- Exchange the code once with the same redirect URI and verifier. Require a
  Bearer access token and parse optional refresh token, expiry, and granted
  scope without logging secrets.
- If `expires_in` is absent, do not invent an expiry; use the token until a
  challenge or explicit refresh. If `scope` is absent, retain the requested
  scopes, and on refresh retain the prior scopes unless replacements are
  returned.
- Store registration and tokens under user/account, resource, issuer, client
  ID, and redirect URI. The token set records its granted scopes. Secret values
  use redacted debug formatting.

## Refresh, retry, and disconnect

- Default refresh skew is 60 seconds and is configurable.
- Concurrent callers for one credential share one refresh; unrelated
  credentials do not block each other.
- A rotated refresh token atomically replaces the old token set. If a refresh
  response omits `refresh_token`, retain the previous one.
- `invalid_grant` removes the unusable token set. An explicit refresh returns
  `InteractionRequired`; an auth-hook refresh leaves the header absent so the
  MCP server can return an authoritative typed challenge. Transient
  discovery/token errors preserve credentials and do not send a known-expired
  token.
- A 401 after one successful refresh or interactive authorization is surfaced
  to the host. A denied incremental scope is cached for the current connection
  attempt to prevent prompt loops.
- Disconnect best-effort revokes tokens only when a validated endpoint is
  available, then always removes local tokens. Cached dynamic registration is
  retained by default and may be removed locally through explicit host policy;
  v1 does not remotely manage registrations. Local deletion is not contingent
  on network success.

## Public boundaries

All impure behavior is trait-backed and Unimock-enabled under tests, doctests,
or `test-support`.

```rust
pub trait McpOAuthManager: Send + Sync {
    async fn authorize(
        &self,
        challenge: &McpAuthorizationChallenge,
        context: &OAuthAuthorizationContext,
    ) -> Result<OAuthConnection>;
    async fn refresh(&self, key: &OAuthCredentialKey)
        -> Result<OAuthConnection>;
    async fn disconnect(&self, key: &OAuthCredentialKey) -> Result<()>;
}

pub trait OAuthCredentialStore: Send + Sync {
    async fn load_registration(&self, key: &OAuthRegistrationKey)
        -> Result<Option<OAuthClientRegistration>>;
    async fn save_registration(
        &self,
        key: &OAuthRegistrationKey,
        value: &OAuthClientRegistration,
    ) -> Result<()>;
    async fn load_tokens(&self, key: &OAuthCredentialKey)
        -> Result<Option<OAuthTokenSet>>;
    async fn save_tokens(
        &self,
        key: &OAuthCredentialKey,
        value: &OAuthTokenSet,
    ) -> Result<()>;
    async fn delete_tokens(&self, key: &OAuthCredentialKey) -> Result<()>;
    async fn delete_registration(&self, key: &OAuthRegistrationKey)
        -> Result<()>;
}

pub trait OAuthUserAgent: Send + Sync {
    async fn authorize(&self, request: OAuthUserAuthorizationRequest)
        -> Result<OAuthAuthorizationResponse>;
}

pub trait AuthorizationServerSelector: Send + Sync {
    async fn select(
        &self,
        resource: &str,
        issuers: &[String],
    ) -> Result<String>;
}
```

`OAuthHttpTransport` separately models bounded GET JSON, POST JSON, and POST
form requests. `OAuthClock` and `OAuthRandom` abstract time and
cryptographically secure bytes. Production implementations are injected into
`DefaultMcpOAuthManager`, which implements `McpOAuthManager`.

`RefreshingMcpAuth` is bound to a canonical resource and credential identity
and implements `json_http::JsonHttpAuth`. It may load or refresh credentials
but must never invoke `OAuthUserAgent`. With no credential it leaves headers
unchanged so `ai-mcp` can obtain the authoritative challenge.

## Typed data and errors

Public DTOs include protected-resource metadata, authorization-server metadata,
client registration, authorization context/response, credential and
registration keys, token set, granted scopes, and connection summary. Known
states use enums; arbitrary JSON is confined to unknown metadata fields at the
wire boundary.

The public `Error` uses typed variants for invalid URLs, resource/issuer
mismatch, unsafe network targets, discovery status/schema failures, issuer
selection cancellation, missing registration, registration rejection, user
denial, callback timeout, state mismatch/reuse, token rejection,
`InvalidGrant`, `InteractionRequired`, store failure, and internal failure.
Errors and diagnostics never contain authorization codes or token values.

## Security requirements

- Require HTTPS for production discovery and OAuth endpoints; permit HTTP only
  for explicit loopback development.
- Reject user info, fragments, dangerous schemes, private/reserved/link-local
  destinations, and disallowed ports according to the injected URL policy.
- Disable automatic redirects; validate scheme, resolved destination, and
  policy at every hop. Pin the connection to validated addresses or verify the
  connected peer so DNS cannot change between validation and use.
- Open authorization URLs with platform APIs, never shell execution.
- Bound response bytes, redirect count, callback lifetime, and request time.
- Never log, serialize to diagnostics, or expose through `Debug` any access
  token, refresh token, authorization code, verifier, or client credential.
- Never send a token to a resource other than its exact credential key.

## Verification

Unit tests cover canonicalization, well-known path insertion, metadata
validation, issuer selection, registration precedence, DCR, PKCE vectors,
state lifecycle, scope minimization, token parsing, expiry skew, refresh
rotation, single-flight behavior, and redaction.

Integration tests use in-process fake resource and authorization servers for:

- initial 401 → discovery → DCR → browser callback → token → MCP retry;
- stored-token reuse and concurrent refresh;
- `invalid_grant` followed by explicit reauthorization;
- 403 incremental scope consent and denial-loop prevention;
- malicious discovery URLs, DNS/private targets, and redirect chains.

No test uses real credentials. A manual live test is ignored and environment
gated if a stable OAuth-enabled MCP test server becomes available.

## Acceptance criteria

- A host can authorize a new remote MCP server and retry initialization once.
- Every MCP request, including side-response POSTs and DELETE, receives the
  same resource-bound Bearer token through `JsonHttpAuth`.
- Refresh is concurrency-safe and rotated tokens are stored atomically.
- Interactive browser work occurs only after an explicit host call.
- 401, 403, user denial, invalid grant, and unsafe discovery remain distinct.
- All required workspace checks and tests pass.
