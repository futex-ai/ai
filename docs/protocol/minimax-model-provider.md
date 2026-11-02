# MiniMax Model Provider Protocol

## Purpose

Add MiniMax as a first-class `ai_interface::Model` provider without treating
its API as an interchangeable OpenAI clone. The adapter uses MiniMax's
OpenAI-compatible Chat Completions endpoint while preserving MiniMax-specific
interleaved-thinking context, error codes, model metadata, and usage details.

## Status

Implemented. The
[completed MiniMax model provider plan](../../plans/minimax-model-provider.md)
records the implementation and verification milestones.

Once implemented, the
[Chat Completions dialects protocol](chat-completions-dialects.md) governs the
shared engine that serves this provider.

## Ownership

- `ai-interface` owns the model spec and routing metadata types and typed
  MiniMax conversation context required for stateless replay.
- `ai-models-compat` owns the stable `minimax` provider id, the MiniMax model
  specs and custom defaults, request and response DTOs, authentication,
  transport dispatch, provider error handling, and response normalization.
- `ai-models-core` continues to own provider-agnostic HTTP classification,
  the capability check, structured-output validation, and pricing wrappers.
- Composition roots own credential lookup, retry/concurrency/pricing wrappers,
  route ordering, and selection of a model spec.

The provider crate must not read environment variables, load configuration, or
resolve secrets.

## API Boundary

The V1 adapter uses:

- Endpoint: `POST https://api.minimax.io/v1/chat/completions`
- Authentication: `Authorization: Bearer <api-key>`
- Content type: `application/json`
- Response mode: SSE, mapped to model events as the
  [model stream protocol](model-stream.md) defines
- Provider identifier: `minimax`

`MiniMaxModel::new` accepts an injected `DynJsonHttpClient`, a model spec,
and an API key. `MiniMaxModel::with_auth` and `with_catalog_auth` accept the
client, a model spec, and an injected `DynJsonHttpAuth`. The request carries
the effort.

The international endpoint is the V1 boundary. The China-region endpoint,
custom endpoint overrides, server-side tools, and the MiniMax Responses and
Anthropic-compatible APIs are out of scope.

## Known Model Specs

The initial catalog contains one spec for each current, non-legacy agent
model. The [control matrix](model-capabilities.md#control-matrix) gives the
efforts, the default effort, the max output, and the context window of each
spec.

| Text id | Constant | Intelligence | Speed | Cost | Features |
| --- | --- | --- | --- | --- | --- |
| `minimax/MiniMax-M3` | `MINIMAX_M3` | Nine | Medium | Low | tools, structured output, vision, video input, long context, reasoning |
| `minimax/MiniMax-M2.7` | `MINIMAX_M2_7` | Eight | Medium | Low | tools, structured output, long context, reasoning |
| `minimax/MiniMax-M2.7-highspeed` | `MINIMAX_M2_7_HIGHSPEED` | Eight | Fast | Medium | tools, structured output, long context, reasoning |

Legacy M2.5, M2.1, and M2 models and the dialogue-specific M2-her model are
not included. A model that the catalog does not list uses a
[custom spec](model-capabilities.md#custom-specs) with the MiniMax custom
defaults.

The MiniMax-M3 spec lists the disabled and medium efforts, and its default
effort is medium. A request with the disabled effort sends the same
`MiniMax-M3` provider model id with a different thinking control. The
routing tiers describe each model at its default medium effort. The tiers
above must be covered by tests; cost is coarse routing metadata, not a
hard-coded billing schedule.

## Request Mapping

Every request must:

1. Send the selected provider model id.
2. Set `stream: true`, `stream_options.include_usage: true`, and
   `reasoning_split: true`.
3. Add a nonblank normalized system prompt as the first `system` message and
   omit empty or whitespace-only system prompts.
4. Append retained conversation messages in order.
5. Send function tools using their name, description, and JSON Schema
   parameters.
6. Omit optional collections and fields when they are empty or unavailable.

Conversation roles map as follows:

- `User` -> `user`
- `Assistant` -> `assistant`
- `Tool` -> `tool`

Plain content is sent as a string. Shared text, image, and video content parts
are sent as OpenAI-compatible `text`, `image_url`, and `video_url` parts.
Base64 images and videos use a `data:<mime-type>;base64,<data>` URL. MiniMax-M3
supports video input, and only its spec advertises the shared
`video_input` feature; the
[video input protocol](video-input.md) defines the shared contract.

Assistant history must include visible content, normalized tool calls, and any
retained MiniMax reasoning context. Tool results include `content` and the
provider tool-call id but no unsupported name field. Tool `content` remains
present when the result is an empty string; only unavailable assistant content
is omitted. Provider context owned by another adapter must not be serialized
as MiniMax reasoning.

MiniMax does not support legacy `function_call`; the adapter sends and accepts
modern `tools` and `tool_calls` only.

Portable temperature and top-p map to their native fields, and an output limit
maps to `max_completion_tokens`. All MiniMax specs support portable `none`
and `auto`. MiniMax-M3 additionally maps strict `Required` and
`RequiredOrAuto` to `tool_choice: "required"`. The other specs retain
strict rejection for `Required`, while `RequiredOrAuto` preserves tools and
maps to `auto`. Named-function choices and nonempty stop sequences return typed
`UnsupportedControl` before transport. The specs carry these rules as spec
data, and the shared capability check applies them. A total timeout replaces
the default 3,600-second overall SSE deadline; the idle timeout is 120 seconds.
`PreferDeferred` falls back to synchronous,
and `RequireDeferred` is unsupported.

MiniMax-M3 streams standard incremental visible-content deltas. MiniMax-M2.x
may stream cumulative snapshots or incremental fragments, so the adapter
infers the shape independently for each stream while withholding visible
content from shared accumulation. A nonempty value that strictly extends the
retained text replaces it, and extending a nonempty retained prefix records
snapshot evidence. A value equal to the retained text is a repeated fragment
before that evidence and a repeated snapshot afterward. Before that evidence, every other nonempty value
appends as a fragment. After snapshot evidence, an otherwise ambiguous value
is a replacing snapshot. Empty content is ignored and never clears retained
text. The retained M2.x text is restored after terminal validation and emitted
as one text delta before the usage event. The adapter also retains the last
nonempty `reasoning_details` snapshot as canonical replay state. A MiniMax-M3
required-tool stream observed by the credentialed suite on 2026-08-22 used
incremental content and revised earlier reasoning text, so M3 content is
accumulated directly and reasoning snapshots replace prior snapshots. Final
usage is required; the stream may end with `[DONE]` or with a clean EOF after
the choices, the finish reason, the usage, and the tool-call metadata
validate.

Append-only `delta.reasoning_content` fragments give reasoning deltas as
they arrive. The revisable `reasoning_details` snapshots give no reasoning
deltas and stay replay state, because a delta cannot replace earlier text.

MiniMax's public reference currently enumerates only `none` and `auto`. Firna
reported a successful live MiniMax-M3 request with a real tool and
`tool_choice: "required"`; this adapter deliberately retains that verified
provider behavior instead of rejecting it from documentation absence alone.
The credentialed repository suite repeats the forced-tool request and asserts
the returned `live_probe` call whenever `LIVE_MODEL_API_KEY` is available.

## Thinking And Replay

MiniMax-M3 thinking maps from the resolved effort:

- Disabled -> `{ "thinking": { "type": "disabled" } }`
- Medium -> `{ "thinking": { "type": "adaptive" } }`
- The capability check lowers low to disabled
- The capability check lowers high, extra high, and max to medium

M2.x reasoning cannot be disabled. The M2.7 specs therefore list only the
medium effort. The capability check lowers high, extra high, and max to
medium. A request with the disabled or low effort fails with
`UnsupportedControl` for effort before transport. Responses record the
resolved effort.

With `reasoning_split: true`, the provider may return both
`reasoning_content` and `reasoning_details`. A reasoning detail preserves all
populated provider fields:

- `type`
- `id`
- `format`
- `index`
- `text`

`ai-interface` stores these values in a typed MiniMax provider-conversation
item. The item is attached to the normalized assistant response and serialized
back onto the corresponding assistant history message on the next request.
This replay is mandatory for tool continuations because MiniMax requires the
complete assistant response to maintain interleaved thinking.

Reasoning text is provider replay state. It must not be concatenated into
`assistant_message`, surfaced as ordinary assistant content, or copied into
normal user-visible logging. The tool runtime therefore removes the complete
MiniMax replay item from request and response copies passed to model-call
loggers while preserving it in the actual model request and retained
conversation.

## Responses And Errors

The [MiniMax response protocol](minimax-responses.md) defines structured
output, response mapping, usage, and error handling, including the
`base_resp` status codes.

## Verification Contract

Unit tests use `JsonHttpTransportMock` through
`TransportBackedJsonHttpClient` and must not perform live network calls.
Coverage includes:

- text ids, provider model ids, context windows, features, and efforts
- provider identity and serde round trips
- endpoint and bearer/custom authentication
- text, image, video, assistant, tool-call, and tool-result request
  serialization
- thinking control and reasoning-split serialization
- reasoning-context parsing, non-disclosure, serde, and continuation replay
- multiple tool calls, malformed arguments, and terminal-call suppression
- every normalized finish reason and empty-choice behavior
- structured-output prompting, success, and validation failures
- strict and fallback tool-choice behavior, including MiniMax-M3 required
- cached/reasoning usage normalization and missing usage
- HTTP and `base_resp` error classification
- credential-free provider construction in `cargo xtask smoke-test`

The ignored workspace integration test `xtask/tests/live_models.rs` separately
calls each listed effort of every MiniMax spec through the production
adapter when an explicit `LIVE_MODEL_API_KEY` is supplied. GitHub Actions
runs that billable suite for eligible pull requests and from the
scheduled/manual `Live model APIs` workflow. Before the catalog probe, it verifies MiniMax-M3 with a real
tool and strict required selection. The implementation environment for this
change had no MiniMax credential, so that billable assertion could not be
re-run locally; deterministic wire coverage and the credentialed workflow
remain mandatory.

The full workspace must pass formatting, Clippy, tests, the Rust file-length
lint, smoke tests, and `cargo xtask check`.

## Official References

- [Model invocation](https://platform.minimax.io/docs/guides/text-generation)
- [OpenAI Chat Completions API](https://platform.minimax.io/docs/api-reference/text-chat-openai)
- [Tool use and interleaved thinking](https://platform.minimax.io/docs/guides/text-m3-function-call)
- [Error codes](https://platform.minimax.io/docs/api-reference/errorcode)
- [Pay-as-you-go pricing](https://platform.minimax.io/docs/guides/pricing-paygo)
