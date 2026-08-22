# ai

Standalone Rust workspace for AI interfaces, model-provider adapters,
provider-agnostic model policy, model fallback routing, JSON HTTP transport, and
in-memory tool-calling runtime behavior.

## Features

- Shared `ai-interface` contracts for conversations, model calls, audio
  transcription, one-image generation and editing, one-video generation,
  tool calls, routing, logging, usage metering, and bounded model-visible tool
  output envelopes
- Typed provider-neutral generation controls, including explicit
  required-with-automatic-fallback tool selection, per-call deadlines, and
  completion preferences, with every provider adapter owning its native wire
  mapping and XAI owning deferred submission and polling
- Provider adapters for Anthropic, DeepSeek, Google Gemini, Kimi, MiniMax,
  OpenAI, QwenCloud, and xAI models, including provider-specific tools,
  reasoning replay, vision where supported, Google and MiniMax inline video
  input, OpenAI and Gemini image generation, OpenAI Sora and Google Veo video
  generation, structured output, usage normalization, catalog-aware
  thinking-level downgrades, and typed errors
- Provider-agnostic wrappers for retry, concurrency, structured output
  validation, known-model catalogs, and usage pricing
- Ordered fallback model composition through `ai-models-multi`
- Trait-backed JSON HTTP client support through `json-http`
- In-memory tool-calling runtime through `ai-tool-calling`, including
  universal tool output management with inline envelopes, stored output ids,
  UTF-8-safe windows, and degraded-window fallbacks
- Streamable HTTP MCP tool discovery and dispatch through `ai-mcp`
- Host-side MCP OAuth discovery, PKCE, refresh, and request authentication
  through `ai-mcp-oauth`

## Protocols

- [Model completion streaming](docs/protocol/model-completion-streaming.md)
  defines internal SSE accumulation, idle and overall deadlines, interruption
  retry policy, provider mappings, response parity, and the Firna revision-bump
  contract.
- [Provider call controls](docs/protocol/provider-call-controls.md) defines
  portable generation and execution intent, provider compatibility, blank
  system handling, full Google schemas, and XAI deferred completion.
- [Image generation](docs/protocol/image-generation.md) defines the shared
  one-image generation/editing boundary and the OpenAI and Google mappings.
- [Video generation](docs/protocol/video-generation.md) defines the shared
  one-video generation boundary, portable controls, and asynchronous OpenAI
  and Google mappings.
- [Live image API tests](docs/protocol/live-image-api-tests.md) defines the
  implemented credentialed catalog coverage, low-cost probe, response
  validation, and CI secret boundary for image providers.
- [Live video API tests](docs/protocol/live-video-api-tests.md) defines the
  implemented credentialed video catalog coverage, shortest portable probe,
  MP4 validation, and CI secret boundary.
- [Video input](docs/protocol/video-input.md) defines the shared video content
  part, the Google and MiniMax mappings, and typed rejection elsewhere.
- [DeepSeek model provider](docs/protocol/deepseek-model-provider.md) defines
  the DeepSeek V4 Pro/Flash catalog, text-only request boundary, thinking,
  replay, tool-calling, JSON-object, usage, and error contract.
- [Kimi model provider](docs/protocol/kimi-model-provider.md) defines the
  implemented Kimi K3 catalog, request, replay, tool-calling,
  structured-output, usage, and error contract.
- [MiniMax model provider](docs/protocol/minimax-model-provider.md) defines the
  provider identity, catalog, request/replay, response, usage, and
  error-normalization contract.
- [Qwen model provider](docs/protocol/qwen-model-provider.md) defines the stable
  Qwen 3.7 Max/Plus/Flash catalog, thinking, vision, replay, tool-calling,
  structured-output, usage, and error contract.
- [Live model API tests](docs/protocol/live-model-api-tests.md) define the
  credentialed provider/catalog coverage, assertions, CI schedule, and secret
  boundary.
- [Tool output management](docs/protocol/tool-output-management.md) defines the
  universal output-id, bounded-envelope, pagination, and raw-output isolation
  contract for tool calls.
- [AI MCP client and tool adapter](docs/protocol/ai-mcp.md) defines streamable
  HTTP transport, session lifecycle, protocol mapping, and tool adaptation.
- [Host-side MCP OAuth](docs/protocol/mcp-oauth.md) defines discovery,
  registration, PKCE, token lifecycle, and request authentication.

## Interfaces

The workspace is library-first. Consumers depend on the crate matching the
boundary they need:

- `ai-interface`: shared DTOs, traits, mocks, error contracts, and
  model-visible tool output envelopes
- `ai-models-core`: reusable model wrappers and provider helper logic
- `ai-models-anthropic`: Anthropic model adapter
- `ai-models-deepseek`: DeepSeek V4 Pro/Flash model adapter and known-model
  catalog
- `ai-models-google`: Google Gemini chat/image and Veo video adapters
- `ai-models-kimi`: Kimi K3 model adapter
- `ai-models-minimax`: MiniMax Chat Completions model adapter and known-model
  catalog
- `ai-models-openai`: OpenAI model, transcription, image, and video adapters
- `ai-models-qwen`: Qwen 3.7 Max/Plus/Flash Chat Completions adapter and
  known-model catalog
- `ai-models-xai`: xAI model adapter
- `ai-models-multi`: ordered fallback model adapter
- `ai-tool-calling`: in-memory tool-calling runtime with output policy, output
  store integration, and the intrinsic `tool_output_read` reader
- `ai-mcp`: MCP 2025-06-18/2025-03-26 streamable HTTP client and
  `ai-interface::Tool` adapter
- `ai-mcp-oauth`: host-side OAuth companion with injected browser, secure
  storage, issuer-selection, clock, randomness, and transport boundaries
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

The smoke test constructs every provider adapter with placeholder credentials
and exercises the in-memory tool runtime. It performs no provider requests and
does not require credentials or network access.

Credentialed model checks live in `xtask/tests/live_models.rs`. They construct
provider adapters only at the composition boundary, then call every
chat-capable catalog variant through the same `DynModel` and
`ToolCallingRuntime` path with portable controls. To test one provider against
the real API, set `LIVE_MODEL_API_KEY` and run the corresponding ignored test,
for example:

```sh
LIVE_MODEL_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_models \
  openai_catalog -- --ignored --exact --nocapture
```

These calls are billable. The dedicated `Live model APIs` workflow runs all
eight provider catalogs for same-repository pull requests, every day, and on
manual dispatch. It requires the repository Actions secrets
`ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY`,
`GOOGLE_API_KEY`, `KIMI_API_KEY`, `MINIMAX_API_KEY`, `OPENAI_API_KEY`,
`QWEN_API_KEY`, and `XAI_API_KEY`. On an eligible run, a missing secret fails
its provider job.

Credentialed image checks live in `xtask/tests/live_images/mod.rs`. They select
every Google and OpenAI catalog entry advertising `ImageGeneration`, construct
the production adapter behind `DynImageGenerator`, and request one square image
per attempt. Run the credential-free guards or one billable provider catalog
with:

```sh
cargo test --locked -p xtask --test live_images
LIVE_IMAGE_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask \
  --test live_images catalog_tests::openai_image_catalog \
  -- --ignored --exact --nocapture
```

The `Live image APIs` workflow runs both provider catalogs for trusted
same-repository pull requests, daily, and on manual dispatch. It uses
`GOOGLE_API_KEY` and `OPENAI_API_KEY`, runs providers sequentially, and may make
up to three billable attempts per catalog model after transient failures.

Credentialed video checks live in `xtask/tests/live_videos/mod.rs`. They select
every Google and OpenAI catalog entry advertising `VideoGeneration`, construct
the production adapter behind `DynVideoGenerator`, and request one four-second
720p landscape MP4. Run the credential-free guards or one billable provider
catalog with:

```sh
cargo test --locked -p xtask --test live_videos
LIVE_VIDEO_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask \
  --test live_videos catalog_tests::openai_video_catalog \
  -- --ignored --exact --nocapture
```

The `Live video APIs` workflow runs both provider catalogs sequentially for
trusted same-repository pull requests, daily, and on manual dispatch. It uses
`GOOGLE_API_KEY` and `OPENAI_API_KEY`. Video jobs are not automatically retried
after submission because a transport failure can leave a billable job running.

Run local AI review after checks pass and the branch has been pushed:

```sh
cargo xtask review
```

## Key Code

- `Cargo.toml`: workspace membership and shared internal crate dependencies
- `crates/ai-interface`: shared AI contracts, including
  call controls and `src/output/` envelope DTOs
- `crates/ai-models-core`: provider-agnostic model wrappers and helpers
- `crates/ai-models-deepseek`: DeepSeek V4 catalog, typed client, thinking,
  request/replay, structured-output, response, usage, and error mapping
- `crates/ai-models-kimi`: Kimi K3 catalog, client, request, replay, response,
  and usage mapping
- `crates/ai-models-minimax`: MiniMax catalog plus request, replay, response,
  usage, and provider-error mapping
- `crates/ai-models-qwen`: Qwen 3.7 catalog, typed client, thinking, vision,
  request/replay, structured output, usage, and error mapping
- `crates/ai-models-*`: concrete provider and fallback adapters
- `crates/ai-tool-calling`: in-memory tool-calling runtime, including
  `src/policy.rs`, `src/output_store/`, and the intrinsic output reader
- `crates/ai-mcp`: streamable HTTP MCP protocol client, authorization
  challenges, and tool adapter
- `crates/ai-mcp-oauth`: protected-resource discovery, public-client
  registration, PKCE authorization, token lifecycle, and MCP auth hook
- `crates/json-http`: HTTP client abstraction used by provider crates
- `xtask/`: local automation for checks, smoke tests, file-length lint, and
  review
- `docs/protocol/tool-output-management.md`: normative universal tool output
  management contract
- `docs/protocol/image-generation.md`: normative shared image generation and
  provider mapping contract
- `docs/protocol/live-image-api-tests.md`: implemented credentialed image-provider
  catalog and CI verification contract
- `docs/protocol/video-generation.md`: normative shared video generation and
  asynchronous provider mapping contract
- `docs/protocol/live-video-api-tests.md`: implemented credentialed video-provider
  catalog and CI verification contract
- `docs/protocol/provider-call-controls.md`: normative model-call control and
  provider wire-compatibility contract
- `docs/protocol/model-completion-streaming.md`: approved internal SSE,
  timeout, interruption, parity, and downstream migration contract
- `docs/protocol/deepseek-model-provider.md`: normative DeepSeek V4 provider
  contract
- `docs/protocol/kimi-model-provider.md`: normative Kimi K3 provider contract
- `docs/protocol/minimax-model-provider.md`: normative MiniMax adapter contract
- `docs/protocol/qwen-model-provider.md`: normative Qwen 3.7 provider contract
- `docs/protocol/`: other normative contracts for shared runtime behavior
- `plans/`: active and completed implementation plans.

## CI

GitHub Actions runs the same Rust verification expected locally on pull requests
and pushes to `main`: formatting, Clippy, tests, Rust file-length lint,
credential-free smoke tests, and `cargo xtask check`. Limiting push-triggered CI
to `main` prevents an open pull request from running the same commit once for
the branch push and again for the pull-request event. The separate `Live model
APIs` workflow makes billable calls through the production adapters for every
chat-capable catalog entry through the generic tool-calling runtime on eligible
pull requests as well as its daily schedule and manual dispatch. The separate
`Live image APIs` workflow exercises every Google and OpenAI image-capable
catalog entry through `DynImageGenerator` with image-specific validation and
sequential provider jobs. The `Live video APIs` workflow similarly exercises
every Google and OpenAI video-capable entry through `DynVideoGenerator`, with
MP4-specific validation and no asset persistence. Forked and Dependabot pull
requests skip all credentialed workflows because GitHub does not provide them
repository Actions secrets.

## Plans

See [plans/README.md](plans/README.md) for active and completed implementation
plans.
