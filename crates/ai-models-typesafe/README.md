# ai-models-typesafe

`ai-models-typesafe` implements TypeSafe System One judgment models behind
`ai_interface::JudgmentModel`. Depend on it when a composition root needs to
evaluate typed condition, choice, or score questions against shared text or
JSON state with explicit credentials.

## Responsibilities

- Implement TypeSafe judgment calls behind the shared `JudgmentModel` trait.
- Own the `typesafe` provider id, the Jev model specs, and judgment-only
  routing metadata.
- Map shared judgment requests and responses to the System One wire contract.
- Apply shared request validation before transport and shared response
  postconditions after normalization.
- Normalize provider token usage and translate transport, HTTP, and malformed
  payload failures into shared typed judgment errors.

## What This Crate Does

`TypeSafeJudgmentModel::from_config(http, spec, config)` accepts an injected
`json-http` client, a `ModelSpec`, and an `ai_models_core::ProviderConfig`.
The config sets the base URL, auth, extra headers, and total timeout.
Requests preserve any base URL path prefix. API keys use Bearer auth.
Auth headers take precedence over extra headers without regard to case.
The existing `new` and `with_auth` constructors remain available and use
the same config path. The crate exports
`PROVIDER_ID`, the Jev specs `JEV_LATEST` and `JEV_1_13_0`, the `CATALOG`
slice, and `CUSTOM_DEFAULTS`. A spec gives its provider model id through
`provider_model_id`. Each spec advertises only `ModelFeature::Judgment`. It
lists no effort and has no default effort, because the adapter has no
reasoning control. The adapter sends one request to
`https://api.typesafe.ai/v1/systemone` for all supplied questions, using a
60-second timeout by default. Endpoint and timeout overrides support alternate
deployments and deterministic tests.

The adapter preserves text and structured JSON content, maps condition
questions to TypeSafe `noul` questions, requires concrete descriptions for
every score criterion, and normalizes condition, choice, and score answers
into the provider-independent interface. It accepts unlisted
provider model ids so callers can pin newer releases before catalog metadata
is updated. It does not read environment variables, load credentials, retry,
price usage, or make network calls during unit tests.

Responses are decoded directly from bytes, and only `2xx` statuses are treated
as successful judgments. Requested answer fragments are decoded independently,
so unknown unrequested answers are ignored, and repeated answer ids, option
labels, or score levels are rejected instead of silently taking the last
value. Authentication hooks are applied exactly once, and the
captured headers are transmitted unchanged. The HTTP client tracks those
headers as auth, so cross-origin redirects omit them. Provider, transport,
malformed-payload, and provider-controlled answer-label diagnostics redact the
explicit API key, every applied authentication header value, and tokens after
the last whitespace in scheme-prefixed values. Hook failures discard their
unsanitizable diagnostic and use the fixed `authentication hook failed`
message.

Every error carries a precise shared kind, the selected adapter identity, and
optional Retry-After and provider request-id values from its actual response.
The adapter tries the optional `x-request-id` and `request-id` header aliases.
`with_clock(Arc<dyn ai_models_core::Clock>)` injects retry-date time.
Production uses `SystemClock`.
Local validation remains before dynamic auth and uses any explicit constructor
key and its last whitespace token to redact known secrets.

## Quick Start

```rust
use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{DynJudgmentModel, JudgmentQuestion, JudgmentRequest};
use ai_models_core::ProviderConfig;
use ai_models_typesafe::{JEV_LATEST, PROVIDER_ID, TypeSafeJudgmentModel};
use json_http::ReqwestJsonHttpClient;

fn build_model(api_key: String) -> DynJudgmentModel {
    Arc::new(TypeSafeJudgmentModel::from_config(
        Arc::new(ReqwestJsonHttpClient::new()),
        JEV_LATEST,
        ProviderConfig::new(PROVIDER_ID, api_key),
    ))
}

fn urgent_request() -> JudgmentRequest {
    JudgmentRequest {
        state: "Payouts have failed for three days.".into(),
        questions: BTreeMap::from([(
            "is_urgent".to_owned(),
            JudgmentQuestion::condition("Does this need urgent attention?", None),
        )]),
    }
}
```

Callers retrieve the API key and inject it at the composition root. Use
`TypeSafeJudgmentModel::with_auth` when credentials come from a reusable auth
hook, and use `CATALOG` or `known_models()` to discover the supported Jev
specs.

## Development

```sh
cargo test -p ai-models-typesafe --all-features
cargo clippy -p ai-models-typesafe --all-targets --all-features -- -D warnings
```

All tests use injected transports and explicit test auth, so no TypeSafe
credentials or network access are required.

### Key Code

- `src/catalog.rs` - the TypeSafe provider id, the Jev specs, `CATALOG`, and
  custom defaults.
- `src/typesafe/client.rs` - `TypeSafeJudgmentModel::from_config`, local
  validation, and HTTP dispatch.
- `src/typesafe/request.rs` - shared-to-TypeSafe request mapping.
- `src/typesafe/response.rs` - answer, score-key, and usage normalization.
- `src/typesafe/error.rs` - status, transport, and auth error classification
  through the shared `json_http_error_core` helper of `ai-models-core`.
- `src/typesafe/redaction.rs` - transmitted-auth secret derivation and
  longest-first diagnostic redaction.
- `src/typesafe/unique_map.rs` - duplicate-rejecting provider map decoding.

### Related Docs

- [`../../docs/protocol/provider-registry.md`](../../docs/protocol/provider-registry.md)
- [`../../docs/protocol/typesafe-judgment-provider.md`](../../docs/protocol/typesafe-judgment-provider.md)
- [`../../docs/protocol/judgment-boundary.md`](../../docs/protocol/judgment-boundary.md)
- [`../../docs/protocol/live-judgment-api-tests.md`](../../docs/protocol/live-judgment-api-tests.md)
- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/model-policies.md`](../../docs/protocol/model-policies.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../json-http/README.md`](../json-http/README.md)
