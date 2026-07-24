# ai

Standalone Rust workspace for AI interfaces, model-provider adapters,
provider-agnostic model policy, model fallback routing, JSON HTTP transport, and
in-memory tool-calling runtime behavior.

## Features

- Shared `ai-interface` contracts for conversations, model calls, audio
  transcription, tool calls, routing, logging, and usage metering
- Provider adapters for Anthropic, Google Gemini, OpenAI, and xAI models
- Provider-agnostic wrappers for retry, concurrency, structured output
  validation, known-model catalogs, and usage pricing
- Ordered fallback model composition through `ai-models-multi`
- Trait-backed JSON HTTP client support through `json-http`
- In-memory tool-calling runtime through `ai-tool-calling`
- Streamable HTTP MCP tool discovery and dispatch through `ai-mcp`

## Interfaces

The workspace is library-first. Consumers depend on the crate matching the
boundary they need:

- `ai-interface`: shared DTOs, traits, mocks, and error contracts
- `ai-models-core`: reusable model wrappers and provider helper logic
- `ai-models-anthropic`: Anthropic model adapter
- `ai-models-google`: Google Gemini model adapter
- `ai-models-openai`: OpenAI model and transcription adapters
- `ai-models-xai`: xAI model adapter
- `ai-models-multi`: ordered fallback model adapter
- `ai-tool-calling`: in-memory tool-calling runtime
- `ai-mcp`: MCP 2025-06-18/2025-03-26 streamable HTTP client and
  `ai-interface::Tool` adapter
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
- `crates/ai-interface`: shared AI contracts
- `crates/ai-models-core`: provider-agnostic model wrappers and helpers
- `crates/ai-models-*`: concrete provider and fallback adapters
- `crates/ai-tool-calling`: in-memory tool-calling runtime
- `crates/ai-mcp`: streamable HTTP MCP protocol client, authorization
  challenges, and tool adapter
- `crates/json-http`: HTTP client abstraction used by provider crates
- `xtask/`: local automation for checks, smoke tests, file-length lint, and
  review
- `docs/protocol/`: approved behavior contracts for planned and implemented
  protocol surfaces
- `plans/`: active and completed implementation plans.

## CI

GitHub Actions runs the same Rust verification expected locally on pull requests
and branch pushes: formatting, Clippy, tests, Rust file-length lint,
credential-free smoke tests, and `cargo xtask check`.

## Protocol Docs

- [AI MCP client and tool adapter](docs/protocol/ai-mcp.md)
- [Host-side MCP OAuth](docs/protocol/mcp-oauth.md)

## Plans

See [plans/README.md](plans/README.md) for active and completed migration plans.
