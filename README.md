# ai

Standalone Rust workspace for AI interfaces, model-provider adapters,
provider-agnostic model policy, model fallback routing, JSON HTTP transport, and
in-memory tool-calling runtime behavior.

## Features

- Shared `ai-interface` contracts for conversations, model calls, audio
  transcription, tool calls, routing, logging, usage metering, and bounded
  model-visible tool output envelopes
- Provider adapters for Anthropic, Google Gemini, OpenAI, and xAI models
- Provider-agnostic wrappers for retry, concurrency, structured output
  validation, known-model catalogs, and usage pricing
- Ordered fallback model composition through `ai-models-multi`
- Trait-backed JSON HTTP client support through `json-http`
- In-memory tool-calling runtime through `ai-tool-calling`, including
  universal tool output management with inline envelopes, stored output ids,
  UTF-8-safe windows, and degraded-window fallbacks

## Protocols

- [Tool output management](docs/protocol/tool-output-management.md) defines the
  universal output-id, bounded-envelope, pagination, and raw-output isolation
  contract for tool calls.

## Interfaces

The workspace is library-first. Consumers depend on the crate matching the
boundary they need:

- `ai-interface`: shared DTOs, traits, mocks, error contracts, and
  model-visible tool output envelopes
- `ai-models-core`: reusable model wrappers and provider helper logic
- `ai-models-anthropic`: Anthropic model adapter
- `ai-models-google`: Google Gemini model adapter
- `ai-models-openai`: OpenAI model and transcription adapters
- `ai-models-xai`: xAI model adapter
- `ai-models-multi`: ordered fallback model adapter
- `ai-tool-calling`: in-memory tool-calling runtime with output policy, output
  store integration, and the intrinsic `tool_output_read` reader
- `json-http`: typed JSON and multipart HTTP client boundary
- `xtask`: repository automation invoked with `cargo xtask ...`

## Developer Get Started

```sh
cargo metadata --format-version 1 --no-deps
cargo xtask check
```

Targeted checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
```

Run local AI review after checks pass and the branch has been pushed:

```sh
cargo xtask review
```

## Key Code

- `Cargo.toml`: workspace membership and shared internal crate dependencies
- `crates/ai-interface`: shared AI contracts, including
  `src/output/` envelope DTOs
- `crates/ai-models-core`: provider-agnostic model wrappers and helpers
- `crates/ai-models-*`: concrete provider and fallback adapters
- `crates/ai-tool-calling`: in-memory tool-calling runtime, including
  `src/policy.rs`, `src/output_store/`, and the intrinsic output reader
- `crates/json-http`: HTTP client abstraction used by provider crates
- `xtask/`: local automation for checks, smoke tests, file-length lint, and
  review
- `docs/protocol/tool-output-management.md`: normative universal tool output
  management contract
- `docs/protocol/`: other normative contracts for shared runtime behavior
- `plans/`: active and completed implementation plans.

## CI

GitHub Actions runs the same Rust verification expected locally on pull requests
and branch pushes: formatting, Clippy, tests, Rust file-length lint,
credential-free smoke tests, and `cargo xtask check`.

## Plans

See [plans/README.md](plans/README.md) for active and completed implementation
plans.
