# xtask

`xtask` contains repository automation for the AI workspace. Depend on it only
through `cargo xtask ...` commands when running local verification, smoke tests,
file-length audits, or AI review.

## Responsibilities

- Run the standard local verification sequence
- Enforce the Rust file-length cap for `crates/` and `xtask/`
- Run a credential-free smoke test for provider construction, tool-calling
  registration, MCP tools, and the resource-bound MCP OAuth hook
- Host opt-in integration tests for every provider catalog against real APIs
- Delegate local AI review to the Codex CLI

## What This Crate Does

The crate exposes the `check`, `rust-file-length-lint`, `smoke-test`, and
`review` commands. `check` runs formatting, Clippy, tests, the file-length lint,
and the smoke test in the same order expected by CI.

`smoke-test` constructs the Anthropic, DeepSeek, Google Gemini, Kimi, MiniMax,
OpenAI, QwenCloud, and xAI model adapters plus the OpenAI transcriber with
placeholder credentials. It also runs an in-memory tool-output pagination
flow. Provider construction does not send network requests or require real
credentials.

`tests/live_models.rs` is a separate, ignored integration suite. Each provider
test reads one explicit `LIVE_MODEL_API_KEY`, constructs every entry returned
by that provider's `known_models()`, sends a minimal real completion, and
validates normalized provider, catalog, model, thinking, finish, text, tool,
and usage fields. The suite never runs as part of `check` or `smoke-test`.

## Quick Start

```sh
cargo xtask check
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
cargo xtask review

# Billable: tests every OpenAI catalog entry against the real API.
LIVE_MODEL_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_models \
  openai_catalog -- --ignored --exact --nocapture
```

## Development

```sh
cargo test -p xtask
cargo clippy -p xtask --all-targets --all-features
```

### Key Code

- `src/cli.rs` - command-line parser
- `src/check.rs` - local verification command plan
- `src/file_length.rs` - Rust line-count audit
- `src/smoke/` - credential-free provider, MCP, OAuth, and pagination smoke tests
- `tests/live_models.rs` - ignored credentialed tests over every provider catalog
- `src/review.rs` - Codex CLI review delegation

### Related Docs

- [`../README.md`](../README.md)
- [`../docs/protocol/live-model-api-tests.md`](../docs/protocol/live-model-api-tests.md)
- [`../plans/README.md`](../plans/README.md)
