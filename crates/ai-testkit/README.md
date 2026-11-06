# ai-testkit

`ai-testkit` records the behavior of this workspace's chat adapters.
Use it for offline conformance tests and explicit provider stream recording.
Product libraries do not depend on this crate.

## Responsibilities

- Build the same six scenarios for each chat provider.
- Store sanitized HTTP exchanges with their source, the text id of the model
  spec, the request effort, and optional provider base URL.
- Run production adapters against ordered replay transports.
- Compare every sanitized outbound request with its recorded exchange.
- Snapshot the ordered model events of each scenario, then its folded
  response or typed error, without provider credentials.
- Prove that a stored version 1 tool loop, read through the composed
  provider converters, still sends each recorded continuation request.

## What This Crate Does

The providers are Anthropic, OpenAI, Google, DeepSeek, Kimi, Qwen,
MiniMax, and xAI. The runner records every event of `Model::stream` and
folds the events with the fold of `Model::complete`. xAI uses its
synchronous path. No scenario uses a video adapter.

Each provider starts with its first chat spec. The tool loop uses the first
spec whose default can think, when the catalog has one. That default is an
effort above disabled, or the provider default. Each request of a scenario
carries the effort of its recording. A recording without an effort runs at
the default of the spec.
`ProviderModelFactory` builds models through `ai-models-registry` with the
injected HTTP client and `ProviderConfig`. Its `ModelFactory` input accepts
that config. `Provider::catalog` filters the registry's combined catalog to
the selected provider's chat specs. Provider crates retain their public
constructors. `Provider::auth` also remains available for injected auth fixtures.
The scenarios are:

| Filename stem | Request and expected exercise |
| --- | --- |
| `text_turn` | One short text turn. |
| `tool_loop` | A tool call with thinking enabled, then a tool result and final answer. |
| `schema_call` | A response schema with one required boolean field. |
| `image_input` | A text prompt and a valid 32 by 32 base64 PNG. |
| `rate_limit` | A synthetic HTTP 429 opening failure. |
| `stream_cut` | A transport failure after the first progress events. |

The second tool step appends the assistant message that holds every part of
the first response with its provider state. It then appends one tool result
for each call and asks for the final answer.
The runner stops at the first model error and stores its shared `ErrorCore`
and earlier failed attempts. It sanitizes every diagnostic and identity field.
It does not retry or change provider behavior.

Replay compares the method, URL, sanitized body, and all recorded headers.
It also checks for missing or extra requests.
A mismatch names the provider, scenario, and zero-based exchange index.
It shows both requests as pretty JSON.

Recordings live at `recordings/<provider>/<scenario>.json`.
Each JSON file has `source`, `catalog_model_id`, an optional `effort`, and
`exchanges` fields. It can also have an optional `base_url` field.
Replay applies that URL to `ProviderConfig`, including its path prefix.
A trailing slash changes no endpoint. A file without `base_url` uses the
provider default. Every existing synthetic recording names its current base
URL. Recorded requests and responses keep their original values.
`catalog_model_id` holds the `provider/model` text id of
the spec, such as `anthropic/claude-sonnet-5`. The replay runner finds that
text id in a combined catalog of the provider's chat specs.
`source` is `synthetic` for hand-built fixtures and `recorded` for live runs.
The initial set contains 48 synthetic files.
Each synthetic success payload follows the provider's documented response
to the adapter's real request.
DeepSeek and the first Qwen spec reject image input before transport.
Their image recordings therefore contain no HTTP exchanges.

File snapshots live under `tests/_tests_/snapshots/`.
Each snapshot shows the event sequence of a scenario, then its folded
response or its error. They preserve successful responses and remaining
defects. The schema scenarios show text deltas, because a call with a
response schema emits text deltas like any other call.
The events in the snapshots show the provider state of each part. The second
recorded Anthropic and Google tool requests replay the signed thinking blocks
and thought signatures in their original order. Anthropic requests use the
max output of the spec.
Rate-limit errors keep the provider message in the shared error core.
Synthetic fixtures can be refreshed offline by wrapping their replay transport
with the recording transport and running the production adapter. This keeps
`source: "synthetic"` and does not prove live provider acceptance.

The legacy conversion test reads one stored version 1 assistant message for
each provider from `tests/fixtures/version1/`. It reads the message through a
converter from the registry. It replays the `tool_loop` recording with that
converted message
as the assistant message of the second step. Every outbound request must
match its recorded exchange.

Mixed version 1 request regressions use the real adapters with an in-memory
transport. They check native content with only its converter and with all
converters installed. They cover repeated text, native block order, calls
without portable entries, repeated native calls, and OpenAI item order when
a full native item comes first. The recorded requests stay unchanged.
MiniMax regressions check that only the first version 1 reasoning item and
its details reach the request, through conversion and plain serde reads.

The rate-limit fixtures use these official error references.
Messages and request ids are synthetic examples, without account details.

| Provider | Error reference |
| --- | --- |
| Anthropic | [Error shapes and rate limits](https://platform.claude.com/docs/en/api/errors) |
| OpenAI | [Error codes](https://developers.openai.com/api/docs/guides/error-codes) |
| Google | [Gemini troubleshooting](https://ai.google.dev/gemini-api/docs/troubleshooting) |
| DeepSeek | [Error codes](https://api-docs.deepseek.com/quick_start/error_codes/) |
| Kimi | [Common error codes](https://platform.kimi.ai/docs/api/errors) |
| Qwen | [Model Studio error codes](https://www.alibabacloud.com/help/en/model-studio/error-code) |
| MiniMax | [Error codes](https://platform.minimax.io/docs/api-reference/errorcode) |
| xAI | [Debugging errors](https://docs.x.ai/developers/debugging) |

## Quick Start

```sh
# Offline. Requires no provider key or network calls.
cargo test -p ai-testkit

# Billable. Set RECORD_API_KEY in your shell before this command.
cargo xtask record-streams --provider openai
```

The recorder runs text, tool, schema, and image scenarios sequentially.
It replaces only those four files for the selected provider.
It records rejected scenarios too and reports their count.
It makes at most five model calls and never retries a billable call.
Provider rejection can reduce the number of HTTP exchanges.
Rate-limit and stream-cut fixtures remain synthetic.

The `Record provider streams` workflow runs on manual dispatch from the
default branch, or when a same-repository pull request gets the
`record-provider-streams` label. It runs providers one at a time and uploads
their four success-scenario files as artifacts. It never commits them.
The user downloads and reviews those files before adding them to a branch.
Neither the recorder nor this workflow runs during `cargo xtask check`.

## Development

```sh
cargo build -p ai-testkit
cargo test -p ai-testkit
cargo clippy -p ai-testkit --all-targets --all-features

# Generate an intentional snapshot update, then read every changed .snap file.
INSTA_UPDATE=always cargo test -p ai-testkit --test conformance
```

Snapshots describe the supplied recordings. A synthetic two-turn success
does not prove that the live provider accepts the replay state.
Recorded SSE events contain only the events consumed by the adapter.
See `json-http` for the transport boundary's status and failure limits.

### Key Code

- `src/scenario.rs` builds the first request and the tool follow-up request.
- `src/provider.rs` selects chat specs, resolves stored text ids, and constructs
  registry adapters from specs and provider config through `ModelFactory`.
- `src/storage.rs` defines `ScenarioRecording`, `RecordingStorage`, and its JSON
  filesystem implementation.
- `src/runner.rs` implements `ConformanceRunner` with injected adapters and replay.
- `src/event_snapshot.rs` captures typed, sanitized model events for snapshots.
- `src/outcome.rs` defines the event sequences and response or error results.
- `tests/_tests_/conformance_tests.rs` snapshots every available provider scenario.
- `tests/_tests_/legacy_conversion_tests.rs` replays converted version 1 tool
  loops from `tests/fixtures/version1/`.
- `tests/_tests_/mixed_native_replay_tests.rs` checks mixed version 1 native
  requests and the reverse OpenAI item order.
- `tests/_tests_/minimax_legacy_replay_tests.rs` checks first-item reasoning
  replay through both version 1 read paths.
- `recordings/` holds the sanitized exchange files.

### Related Docs

- [Architecture](../../docs/architecture.md)
- [Model capabilities](../../docs/protocol/model-capabilities.md)
- [json-http](../json-http/README.md)
- [xtask recorder](../../xtask/README.md)
- [Model stream](../../docs/protocol/model-stream.md)
- [Model policies](../../docs/protocol/model-policies.md)
- [Conversation parts](../../docs/protocol/conversation-parts.md)
- [Provider registry](../../docs/protocol/provider-registry.md)
