# ai-models-registry

`ai-models-registry` builds provider adapters from shared model specs and
provider settings. Hosts depend on this crate to select adapters without
repeating provider matches. The host supplies the HTTP client and credentials.

## Responsibilities

- Store one `ProviderConfig` for each provider.
- Build model, image, transcription, and judgment adapters.
- Combine the catalogs and stored-message converters of included providers.
- Return typed construction errors without making a provider request.

## What This Crate Does

`Registry` stores an injected `DynJsonHttpClient`. `with_provider` and `insert`
replace earlier settings for the same provider. `model`, `image`,
`transcription`, and `judgment` return shared trait objects. Each adapter uses
the supplied spec, client, and config. The host adds retry, fallback,
concurrency, and pricing layers after construction.

`ProviderConfig` and `ProviderTimeouts` are exported from `ai-models-core`.
The config supplies the base URL, API key or auth hook, extra headers, and
timeouts. Construction reads no environment variable or secret store.

The default features include all nine providers. Each provider has its own
feature: `anthropic`, `deepseek`, `google`, `kimi`, `minimax`, `openai`, `qwen`,
`typesafe`, and `xai`. The five compatible providers share one optional
`ai-models-compat` dependency. Each feature exposes only its own dialect.

`catalog()` and `Registry::catalog()` combine included catalogs in
alphabetical provider order. They preserve the order within each catalog.
`Registry::converter()` composes included stored-message converters. The
shared converter preserves items from absent providers as provider state.

`BuildError` distinguishes a missing config, a provider that is not included,
and a provider without the requested modality. Video construction remains
pending until the user settles the video contract. This crate exposes no
video builder during that work.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{DynModel, ModelSpec, ProviderId};
use ai_models_registry::{BuildResult, ProviderConfig, Registry};
use json_http::ReqwestJsonHttpClient;

fn build(spec: &ModelSpec) -> BuildResult<DynModel> {
    let registry = Registry::new(Arc::new(ReqwestJsonHttpClient::new()))
        .with_provider(ProviderConfig::new(ProviderId::new("google"), "demo-key"));
    registry.model(spec)
}
```

## Development

```sh
cargo test -p ai-models-registry
cargo test -p ai-models-registry --no-default-features
cargo clippy -p ai-models-registry --all-targets --all-features -- -D warnings
```

### Key Code

- `src/registry.rs`: `Registry`, provider config storage, and adapter builders.
- `src/models.rs`: conversational model construction for each included provider.
- `src/modalities.rs`: image, transcription, and judgment construction.
- `src/providers.rs`: the feature-selected combined catalog.
- `src/error.rs`: `BuildError`, `BuildResult`, and `Modality`.

### Related Docs

- [Architecture](../../docs/architecture.md)
- [Provider registry](../../docs/protocol/provider-registry.md)
- [Model capabilities](../../docs/protocol/model-capabilities.md)
- [Conversation parts](../../docs/protocol/conversation-parts.md)
- [Shared config and adapter helpers](../ai-models-core/README.md)
- [JSON HTTP client](../json-http/README.md)
