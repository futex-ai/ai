# xtask

`xtask` contains repository automation for the AI workspace. Depend on it only
through `cargo xtask ...` commands when running local verification, smoke tests,
file-length audits, or AI review.

## Responsibilities

- Run the standard local verification sequence
- Enforce the Rust file-length cap for `crates/` and `xtask/`
- Regenerate the control matrix of the model capabilities protocol from the
  model specs
- Run a credential-free smoke test for chat, transcription, image-provider,
  video-provider, and TypeSafe judgment-provider construction, tool-calling
  registration, MCP tools, and the resource-bound MCP OAuth hook
- Host credentialed integration tests for every chat, image-provider, and
  judgment-provider catalog against real APIs
- Delegate local AI review to the Codex CLI
- Record sanitized chat scenarios through the production provider adapters

## What This Crate Does

The crate exposes the `check`, `rust-file-length-lint`, `smoke-test`,
`record-streams`, `capability-matrix`, and `review` commands. `check` runs
formatting, Clippy, tests, the file-length lint, and the smoke test in the same
order expected by CI.

`capability-matrix` regenerates the "Control matrix" section of
`docs/protocol/model-capabilities.md` from the provider catalogs and their
custom defaults, and writes the doc in place. Run it after a change to a
model spec or to custom defaults, and commit the result. No CI step or
`check` step compares the committed matrix with the generator output.

`smoke-test` constructs the Anthropic, DeepSeek, Google Gemini, Kimi, MiniMax,
OpenAI, QwenCloud, and xAI model adapters, the OpenAI transcriber, the Google
and OpenAI image generators, the Google video generator, and the TypeSafe Jev
judgment adapter, with placeholder credentials. It also runs an in-memory
tool-output pagination flow. Provider construction does not send network
requests or require real credentials.
Every provider adapter is built through `ai-models-registry`. The transcriber
and the video generator use custom specs, because no catalog lists those
models. Provider crates retain their public constructors.

`tests/live_models.rs` owns the chat live suite and its credential-free guards.
Each ignored provider test reads one explicit `LIVE_MODEL_API_KEY` and takes
every chat-capable spec of that provider in the registry's combined catalog. It runs
one probe at the default of the spec, then one probe for each other listed
effort. The default probe of a spec without a default effort sets no effort.
Each probe sets the requested effort and sends a real one-step turn through
the generic tool-calling runtime. Every provider first gets a synchronous
text-parity probe. The catalog then uses portable
no-tools, ten-minute, `PreferDeferred` controls: seven providers fall back to
their synchronous lifecycle, and xAI uses its deferred lifecycle. Every probe,
also an xAI deferred probe, must emit text deltas. The text parts of those
deltas, joined in part position order, must equal the response text. A text
or reasoning delta can be empty only when it carries provider state. The
suite also validates
normalized provider, text id, model, effort, finish, text, tool, and usage
fields. A successfully completed synchronous probe or catalog probe gets one
bounded retry only when the response misses the requested marker and has no
other response failure; event validation must also be clean apart from possible
terminal-parity failure against that response. This absorbs non-deterministic
wording but does not retry request, identity, finish, tool, usage, or other
event failures, and costs at most one extra billable call per probe.
Image-, video-, and judgment-capable specs are excluded because they
use separate specialized interfaces. Credentialed tests never run as part of
`check` or `smoke-test`; credential-free bridge, validation, catalog, and
workflow guards do. The dedicated GitHub Actions workflow invokes
the ignored tests for eligible pull requests, daily verification, and manual
dispatch. The MiniMax job first sends MiniMax-M3 a real tool with strict
`Required` selection and asserts that the provider returns that tool call.
The judgment workflow additionally requires the repository Actions variable
`LIVE_JUDGMENT_API_TESTS_ENABLED=true`; its provider job remains skipped until
the TypeSafe credential is provisioned and that opt-in is enabled.

`tests/live_images/mod.rs` is the corresponding ignored image suite. It selects
every Google and OpenAI catalog spec advertising `ImageGeneration`, constructs
the registry adapter behind `DynImageGenerator`, and runs the same safe
square, low-quality request through each entry sequentially. Failures whose
error core has the transient or rate limited kind use the shared 100ms/250ms
retry schedule of `ai-models-policy`, for no more than three attempts.
Successful responses must contain matching PNG, JPEG, or WebP bytes and the
expected provider/model identity. Credential-free tests enforce
catalog registration, adapter construction, retry classes, response
validation, and workflow secret/event boundaries.

No live suite covers video generation, because no catalog lists a video model.

`tests/live_judgments/mod.rs` is the corresponding ignored judgment suite. It
selects every TypeSafe catalog spec advertising `Judgment`, constructs the
registry adapter behind `DynJudgmentModel`, and evaluates a short support
message against condition, choice, and score questions. Failures whose error
core has the transient or rate limited kind use the shared retry schedule of
`ai-models-policy` for no more than three attempts. Credential-free guards
cover catalog registration, construction, probe shape, retry behavior,
normalized validation, and workflow boundaries.
None of the live suites makes provider calls during `check` or
`smoke-test`.

`record-streams --provider <name>` reads `RECORD_API_KEY` and records the
text, two-turn tool, schema, and image scenarios from `ai-testkit`.
Provider names are `anthropic`, `openai`, `google`, `deepseek`, `kimi`,
`qwen`, `minimax`, and `xai`. The command uses the first chat spec at its
default. For the tool scenario it uses the first spec whose default can
think: an effort above disabled, or the provider default. Each file stores
the text id of the spec and its default effort, or no effort for the
provider default. xAI stays synchronous.
It writes four files under `crates/ai-testkit/recordings/<provider>/`
with `source: "recorded"`. It preserves errors and reports their count.
It never retries model calls. Rate-limit and stream-cut files stay synthetic.
These billable calls run only when the recorder is invoked explicitly.

The `Record provider streams` workflow runs on manual dispatch from the
default branch and on the `record-provider-streams` label for trusted
same-repository pull requests. It uses the eight existing provider secrets
as `RECORD_API_KEY`, runs one provider at a time, and uploads the four
scenario files as artifacts. It does not commit or push the recordings.
It does not run during ordinary CI or `cargo xtask check`.

## Quick Start

```sh
cargo xtask check
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
cargo xtask capability-matrix
cargo xtask review

# Billable. Set RECORD_API_KEY in your shell first.
cargo xtask record-streams --provider openai

# Credential-free: runs image registry, runner, retry, and workflow guards.
cargo test --locked -p xtask --test live_images

# Credential-free: runs judgment registry, runner, retry, and workflow guards.
cargo test --locked -p xtask --test live_judgments

# Credential-free: runs chat catalog, event, runner, and workflow guards.
cargo test --locked -p xtask --test live_models

# Billable: tests every effort of every chat-capable OpenAI spec against the real API.
LIVE_MODEL_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_models \
  openai_catalog -- --ignored --exact --nocapture

# Billable: tests every image-capable OpenAI catalog spec against the real API.
LIVE_IMAGE_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask --test live_images \
  catalog_tests::openai_image_catalog -- --ignored --exact --nocapture

# Billable: tests every TypeSafe judgment catalog spec against the real API.
LIVE_JUDGMENT_API_KEY="$TYPESAFE_API_KEY" cargo test --locked -p xtask \
  --test live_judgments catalog_tests::typesafe_judgment_catalog \
  -- --ignored --exact --nocapture
```

## Development

```sh
cargo test -p xtask
cargo clippy -p xtask --all-targets --all-features
```

### Key Code

- `src/cli.rs` - command-line parser
- `src/check.rs` - local verification command plan
- `src/capability_matrix/` - spec composition, table generation, and bounded
  updates to the model capability reference
- `src/file_length.rs` - Rust line-count audit
- `src/smoke/` - credential-free provider, MCP, OAuth, and pagination smoke tests
- `src/smoke/providers.rs` - registry construction for chat, image, video,
  transcription, and judgment adapters with placeholder credentials
- `tests/live_models.rs` and `tests/live_models/` - ignored credentialed tests
  plus credential-free event, catalog, runner, tool-choice, and workflow guards
- [`tests/live_models/runner_tests.rs`](tests/live_models/runner_tests.rs) -
  `run_live_provider` builds production probes, and `complete_through_runtime`
  captures responses from one-step agent turns.
- [`tests/live_models/event_tests.rs`](tests/live_models/event_tests.rs) -
  `observing_model` records model-stream events for validation.
- `tests/live_images/mod.rs` - ignored credentialed tests and credential-free
  guards over image-provider catalogs
- `tests/live_judgments/mod.rs` - ignored credentialed tests and
  credential-free guards over judgment-provider catalogs
- `tests/live_images/retry_tests.rs` and `tests/live_judgments/retry_tests.rs` -
  `RetryingImageGenerator` and `RetryingJudgmentModel` apply fixed live-test
  waits after `ErrorKind::RateLimited` or `ErrorKind::Transient`.
  Both read `ai_models_policy::STANDARD_TRANSIENT_RETRY_DELAYS` from
  `../crates/ai-models-policy/src/retry_config.rs`.
- `src/review.rs` - Codex CLI review delegation
- `src/record_streams/` - explicit recording command with injected transport,
  model, scenario runner, and recording storage boundaries

### Related Docs

- [`../README.md`](../README.md)
- [`../crates/ai-testkit/README.md`](../crates/ai-testkit/README.md)
- [`../docs/development.md`](../docs/development.md)
- [`../docs/protocol/model-capabilities.md`](../docs/protocol/model-capabilities.md)
- [`../docs/protocol/live-model-api-tests.md`](../docs/protocol/live-model-api-tests.md)
- [`../docs/protocol/live-image-api-tests.md`](../docs/protocol/live-image-api-tests.md)
- [`../docs/protocol/live-judgment-api-tests.md`](../docs/protocol/live-judgment-api-tests.md)
