# xtask

`xtask` contains repository automation for the AI workspace. Depend on it only
through `cargo xtask ...` commands when running local verification, smoke tests,
file-length audits, or AI review.

## Responsibilities

- Run the standard local verification sequence
- Enforce the Rust file-length cap for `crates/` and `xtask/`
- Run a credential-free smoke test for chat, transcription, and image-provider
  construction, tool-calling registration, MCP tools, and the resource-bound
  MCP OAuth hook
- Delegate local AI review to the Codex CLI

## What This Crate Does

The crate exposes the `check`, `rust-file-length-lint`, `smoke-test`, and
`review` commands. `check` runs formatting, Clippy, tests, the file-length lint,
and the smoke test in the same order expected by CI.

`smoke-test` constructs the Anthropic, DeepSeek, Google Gemini, Kimi, MiniMax,
OpenAI, QwenCloud, and xAI model adapters, the OpenAI transcriber, and the
Google and OpenAI image generators with placeholder credentials. It also runs
an in-memory tool-output pagination flow. Provider construction does not send
network requests or require real credentials.

## Quick Start

```sh
cargo xtask check
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
cargo xtask review
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
- `src/review.rs` - Codex CLI review delegation

### Related Docs

- [`../README.md`](../README.md)
- [`../plans/README.md`](../plans/README.md)
