# xtask

`xtask` contains repository automation for the AI workspace. Depend on it only
through `cargo xtask ...` commands when running local verification, smoke tests,
file-length audits, or AI review.

## Responsibilities

- Run the standard local verification sequence
- Enforce the Rust file-length cap for `crates/` and `xtask/`
- Run a credential-free smoke test for chat, transcription, image-provider,
  and video-provider construction, tool-calling registration, MCP tools, and
  the resource-bound MCP OAuth hook
- Host credentialed integration tests for every chat, image-provider, and
  video-provider catalog against real APIs
- Delegate local AI review to the Codex CLI

## What This Crate Does

The crate exposes the `check`, `rust-file-length-lint`, `smoke-test`, and
`review` commands. `check` runs formatting, Clippy, tests, the file-length lint,
and the smoke test in the same order expected by CI.

`smoke-test` constructs the Anthropic, DeepSeek, Google Gemini, Kimi, MiniMax,
OpenAI, QwenCloud, and xAI model adapters, the OpenAI transcriber, and the
Google and OpenAI image and video generators with placeholder credentials. It
also runs an in-memory tool-output pagination flow. Provider construction does
not send network requests or require real credentials.

`tests/live_models.rs` owns the chat live suite and its credential-free guards.
Each ignored provider test reads one explicit `LIVE_MODEL_API_KEY`, constructs
every chat-capable entry returned by that provider's `known_models()`, erases it
behind `DynModel`, and sends real one-step turns through `ToolCallingRuntime`.
A test-only model wrapper routes those runtime calls through
`complete_with_events`, records ordered deltas, and retains the ordinary
terminal checkpoint. Every provider first gets a synchronous assistant-parity
probe. The catalog then uses portable no-tools, ten-minute, `PreferDeferred`
controls: seven providers retain event parity on their synchronous fallback,
while xAI's deferred lifecycle must remain silent. The suite also validates
normalized provider, catalog, model, thinking, finish, text, tool, and usage
fields. Image- and video-generation entries are excluded because they use
separate specialized interfaces. Credentialed tests never run as part of
`check` or `smoke-test`; credential-free event policy, bridge, validation,
catalog, and workflow guards do. The dedicated GitHub Actions workflow invokes
the ignored tests for eligible pull requests, daily verification, and manual
dispatch. The MiniMax job first sends MiniMax-M3 a real tool with strict
`Required` selection and asserts that the provider returns that tool call.

`tests/live_images/mod.rs` is the corresponding ignored image suite. It selects
every Google and OpenAI catalog entry advertising `ImageGeneration`, constructs
the production adapter behind `DynImageGenerator`, and runs the same safe
square, low-quality request through each entry sequentially. Transient and
rate-limit failures use the shared 100ms/250ms retry schedule, for no more than
three attempts. Successful responses must contain matching PNG, JPEG, or WebP
bytes and the expected provider/model identity. Credential-free tests enforce
catalog registration, adapter construction, retry classes, response
validation, and workflow secret/event boundaries.

`tests/live_videos/mod.rs` is the corresponding ignored video suite. It selects
every Google and OpenAI catalog entry advertising `VideoGeneration`, constructs
the production adapter behind `DynVideoGenerator`, and submits one safe
four-second landscape 720p request per entry sequentially. Successful responses
must contain an MP4 signature, normalized metadata, and expected provider/model
identity. The runner does not automatically retry submissions: a transport
failure can leave the upstream job running, so retrying could create duplicate
billable renders. Credential-free guards cover catalog registration, adapter
construction, request shape, response validation, and workflow boundaries.
None of the three live suites makes provider calls during `check` or
`smoke-test`.

## Quick Start

```sh
cargo xtask check
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
cargo xtask review

# Credential-free: runs image registry, runner, retry, and workflow guards.
cargo test --locked -p xtask --test live_images

# Credential-free: runs video registry, runner, validation, and workflow guards.
cargo test --locked -p xtask --test live_videos

# Credential-free: runs chat catalog, event, runner, and workflow guards.
cargo test --locked -p xtask --test live_models

# Billable: tests every chat-capable OpenAI catalog entry against the real API.
LIVE_MODEL_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_models \
  openai_catalog -- --ignored --exact --nocapture

# Billable: tests every image-capable OpenAI catalog entry against the real API.
LIVE_IMAGE_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_images \
  catalog_tests::openai_image_catalog -- --ignored --exact --nocapture

# Billable: tests every video-capable OpenAI catalog entry against the real API.
LIVE_VIDEO_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_videos \
  catalog_tests::openai_video_catalog -- --ignored --exact --nocapture
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
- `tests/live_models.rs` and `tests/live_models/` - ignored credentialed tests
  plus credential-free event, catalog, runner, tool-choice, and workflow guards
- `tests/live_images/mod.rs` - ignored credentialed tests and credential-free
  guards over image-provider catalogs
- `tests/live_videos/mod.rs` - ignored credentialed tests and credential-free
  guards over video-provider catalogs
- `src/review.rs` - Codex CLI review delegation

### Related Docs

- [`../README.md`](../README.md)
- [`../docs/protocol/live-model-api-tests.md`](../docs/protocol/live-model-api-tests.md)
- [`../docs/protocol/live-image-api-tests.md`](../docs/protocol/live-image-api-tests.md)
- [`../docs/protocol/live-video-api-tests.md`](../docs/protocol/live-video-api-tests.md)
- [`../plans/README.md`](../plans/README.md)
