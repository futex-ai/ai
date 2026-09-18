# ai-models-typesafe

`ai-models-typesafe` implements TypeSafe System One judgment models behind
`ai_interface::JudgmentModel`. Depend on it when a composition root needs to
evaluate typed condition, choice, or score questions against shared text or
JSON state with explicit credentials.

## Responsibilities

- Implement TypeSafe judgment calls behind the shared `JudgmentModel` trait.
- Own Jev catalog ids and judgment-only routing metadata.
- Map shared judgment requests and responses to the System One wire contract.
- Apply shared request validation before transport and shared response
  postconditions after normalization.
- Normalize provider token usage and translate transport, HTTP, and malformed
  payload failures into shared typed judgment errors.

## What This Crate Does

`TypeSafeJudgmentModel` accepts an injected `json-http` client, a provider
model id, and either an explicit API key or auth hook. It sends one request to
`https://api.typesafe.ai/v1/systemone` for all supplied questions, using a
60-second timeout by default. Endpoint and timeout overrides support alternate
deployments and deterministic tests.

The adapter preserves text and structured JSON content, maps condition
questions to TypeSafe `noul` questions, and normalizes condition, choice, and
score answers into the provider-independent interface. It accepts unlisted
provider model ids so callers can pin newer releases before catalog metadata
is updated. It does not read environment variables, load credentials, retry,
price usage, or make network calls during unit tests.

Responses are decoded directly from bytes, and only `2xx` statuses are treated
as successful judgments. Requested answer fragments are decoded independently,
so unknown unrequested answers are ignored without hiding repeated score keys
in requested answers. Provider, transport, auth-hook, and malformed-payload
diagnostics redact the API key and applied authentication header values.

## Quick Start

```rust
use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{DynJudgmentModel, JudgmentQuestion, JudgmentRequest};
use ai_models_typesafe::{JEV_LATEST, TypeSafeJudgmentModel};
use json_http::ReqwestJsonHttpClient;

fn build_model(api_key: String) -> DynJudgmentModel {
    Arc::new(TypeSafeJudgmentModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        JEV_LATEST,
        api_key,
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
hook, and use `known_models()` to discover the supported Jev catalog entries.

## Development

```sh
cargo test -p ai-models-typesafe --all-features
cargo clippy -p ai-models-typesafe --all-targets --all-features -- -D warnings
```

All tests use injected transports and explicit test auth, so no TypeSafe
credentials or network access are required.

### Key Code

- `src/catalog.rs` - known Jev ids and judgment-only routing metadata.
- `src/typesafe/client.rs` - construction, local validation, and HTTP dispatch.
- `src/typesafe/request.rs` - shared-to-TypeSafe request mapping.
- `src/typesafe/response.rs` - answer, score-key, and usage normalization.
- `src/typesafe/error.rs` - status, transport, and auth error classification.
- `src/typesafe/redaction.rs` - applied-auth discovery and diagnostic redaction.

### Related Docs

- [`../../docs/protocol/typesafe-judgment-provider.md`](../../docs/protocol/typesafe-judgment-provider.md)
- [`../../docs/protocol/live-judgment-api-tests.md`](../../docs/protocol/live-judgment-api-tests.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../../plans/add-typesafe-judgment-provider.md`](../../plans/add-typesafe-judgment-provider.md)
