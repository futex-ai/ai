# OpenRouter Model Provider Protocol

## Status And Scope

Planned; this provider is not implemented. The
[implementation plan](../../plans/openrouter-model-provider.md) delivers a
curated `ai-models-openrouter` adapter, the
[model-offering router](model-offerings.md),
[lossless replay](openrouter-replay.md), and
[provider-reported accounting](model-usage-cost.md).

The initial catalog demonstrates one model also available natively and one
model with no native adapter in this workspace. Text, supported image input,
custom tools, reasoning, structured output, and public completion events are
included. Dynamic model discovery, arbitrary endpoints/model ids, model
fallback arrays, automatic/latest/free aliases, server tools, transformations,
image/video generation, audio/video input, and native Responses/Messages API
access are outside this initial adapter.

## Ownership And Construction

`ai-interface` adds `ProviderKind::OpenRouter` with exact config/display/serde
value `openrouter`, plus typed OpenRouter replay data. The provider crate owns
catalog entries, typed wire DTOs, normalization, and profile validation. It
uses injected `DynJsonHttpClient` and `DynJsonHttpAuth`, implements `Model`, and
is consumed through `DynModel`. It does not load credentials or deployment
configuration itself.

All constructors return a typed configuration result.
`OpenRouterModel::new(client, api_key)` selects the disabled Sonnet catalog
entry. `with_auth(client, auth)` selects the same entry.
`with_catalog_auth(client, auth, catalog_id)` returns a typed result and
derives the canonical identity, wire id, thinking level, and upstream profile
from that entry. Unknown ids fail construction before auth/transport. There
is no independent wire-id/thinking override that can disagree with the catalog.

Requests use `POST https://openrouter.ai/api/v1/chat/completions`, bearer auth,
JSON content, and `stream: true`. Default overall/idle deadlines are
3,600/120 seconds; portable `total_timeout` replaces the overall deadline.
See the [Chat Completions API](https://openrouter.ai/docs/api/api-reference/chat/create-a-chat-completion).
The adapter sends neither obsolete usage opt-ins nor attribution headers by
default. Authentication remains explicit and credentials never appear in logs.

## Initial Catalog And Upstream Profiles

These are curated adapter ceilings and internal ranking choices, not promises
to reproduce a provider's dynamic maximum or measured benchmark score.

| Catalog id | Canonical model | Thinking | Profile | Context | Intelligence | Speed | Cost |
| --- | --- | --- | --- | ---: | --- | --- | --- |
| `openrouter-claude-sonnet-4-6` | `ClaudeSonnet46` | Disabled | Anthropic | 200,000 | Nine | Medium | High |
| `openrouter-claude-sonnet-4-6-thinking-high` | `ClaudeSonnet46` | High | Anthropic | 200,000 | Nine | Slow | High |
| `openrouter-llama-3-3-70b-instruct` | `Llama3370bInstruct` | Disabled | Groq | 128,000 | Seven | Fast | Low |

Expose corresponding uppercase constants and `known_models()`. Both Sonnet
entries send `anthropic/claude-sonnet-4.6`; Llama sends
`meta-llama/llama-3.3-70b-instruct`. All entries advertise `ToolCalling`,
`StructuredOutput`, and `LongContext`. Sonnet additionally advertises `Vision`;
only its high entry advertises `Reasoning`.

`OpenRouterUpstream` is a provider-local enum, initially `Anthropic` and
`Groq`, mapped exactly to `anthropic` and `groq`. Each catalog entry fixes its
profile. Every request sends `provider.only` containing that one slug,
`provider.allow_fallbacks: false`, and `provider.require_parameters: true`.
Neither callers nor portable controls can broaden the backend set. This
provides gateway/API-key fallback for native Claude, but does not claim
independent resilience to an outage of the same underlying Anthropic service.

OpenRouter normally permits backend routing and can ignore unsupported
parameters; the restricted profiles are an intentional adapter contract.
See [provider selection](https://openrouter.ai/docs/guides/routing/provider-selection).
The initial metadata was checked against the
[Sonnet endpoints](https://openrouter.ai/api/v1/models/anthropic/claude-sonnet-4.6/endpoints)
and [Llama endpoints](https://openrouter.ai/api/v1/models/meta-llama/llama-3.3-70b-instruct/endpoints)
on 2026-09-08. Implementation must recheck availability/support and stop short
of marking an unavailable profile complete; update this spec explicitly before
substituting models, endpoints, limits, or capabilities.

Future profiles are separate curated offerings when their upstream selection
changes. They require fresh endpoint evidence and conformance tests. No union
of all OpenRouter backends' features may be advertised as one guaranteed set.

## Request And Control Mapping

Nonblank system text becomes the first `system` message; blank system text is
omitted. Preserve authored content, role order, optional names on user and
assistant messages, tool ids, and tool-result strings including empty strings.
Empty assistant content on a tool turn may be null. Sonnet maps ordered text
and base64 images to Chat Completions content parts. Llama accepts text only;
typed images and all video input fail locally rather than being discarded.

| Control | Disabled Sonnet | High Sonnet | Llama |
| --- | --- | --- | --- |
| Thinking | `reasoning.enabled: false` | `reasoning.effort: high` | omit reasoning |
| Temperature / top-p | map | fixed native defaults; omit | map |
| Output limit | `max_tokens` | `max_tokens` | `max_tokens` |
| Ordered stops | `stop`, at most four | `stop`, at most four | `stop`, at most four |
| None / auto tools | map | map | map |
| Required / named tool | map | unsupported | map |
| RequiredOrAuto | required | auto with tools retained | required |

Default output allowance is 4,096 tokens for disabled entries and 8,192 for
high Sonnet. An explicit maximum replaces the default, capped at the adapter
ceiling of 16,384. Zero output allowance and more than four stops return typed
`UnsupportedControl`. High Sonnet rejects an explicit effective allowance of
1,024 or fewer before transport to leave room beyond the documented minimum
thinking budget. Thinking level is an intent category, not an equal token
allocation across native access and OpenRouter. The gateway's effort-to-budget
mapping remains provider-owned; see
[reasoning controls](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens).

`None` omits tool definitions and sends no forced choice. Otherwise nonempty
tools become modern `function` definitions with name, description, and the
complete parameters schema. No provider strict-tool flag or legacy function
API is sent. Named choice must name an offered tool; forced choice with no
tools is a typed local failure. `PreferDeferred` uses synchronous SSE;
`RequireDeferred` returns `UnsupportedControl` before auth or transport.

Both profiles implement structured output through `response_format` set to
`json_object`, an appended schema-specific system instruction, and shared
local JSON Schema validation. Advertised support promises a validated shared
response, not native schema enforcement. Validate only natural stopped
responses with no tool calls. Structured requests emit no public text events,
following the [completion-events contract](model-completion-events.md).

## Generation Metadata

`ai_interface::model_metadata` owns `ProviderGeneration` with
`provider: ProviderKind` and `id: ProviderGenerationId`. The id is a private
owned-string newtype: construction and string serde reject blank values with
a typed error and preserve every accepted value without trimming or rewriting.
Add `ModelResponse.provider_generation: Option<ProviderGeneration>`, defaulting
to `None` during deserialization and omitted when absent. Existing providers
and fixtures initialize it to `None`; OpenRouter populates it for every
successful response, including plain text without replay context.

Map the response envelope's top-level `id` to this field, separately from
model ids, tool ids, and reasoning-detail ids. The
[usage reference](https://openrouter.ai/docs/cookbook/administration/usage-accounting)
identifies this as the generation id. Retain the first nonblank string seen
across SSE chunks; later supplied ids must be identical. Missing/null ids in
individual chunks do not erase it. Blank, non-string, or conflicting ids, and
an entirely absent id at stream completion, are typed invalid-provider-data
failures under the stream interruption policy.

Buffered and event-observing calls return identical metadata. Wrappers and
the router preserve the successful response's value; its provider must agree
with the response provider. The tool runtime forwards it to successful model
logger copies and, for validated responses, `ModelResponseCheckpoint` before
tool dispatch or turn completion. Hosts can persist the serialized response
through these existing hooks. `StepOutcome`, `RunOutcome`, and retained
conversation messages do not acquire this field; no automatic history archive
is added. Each continuation reports its own generation id. It is never replayed
upstream, included in tool-call scope hashing, or emitted as completion text.
Failed/interrupted calls do not return partial metadata, and no generation
lookup is introduced.

## Streaming, Responses, And Errors

Use shared text/tool/usage accumulation where its semantics apply, with a
typed OpenRouter normalization layer retaining fields it currently discards.
Do not copy the OpenAI Responses adapter or deserialize away OpenRouter
reasoning, actual cost, generation id, or returned model identity.

Require a primary choice at index zero, terminal finish reason, final token
usage, and `[DONE]`. Usage-only chunks with no choices are valid. Missing
terminal markers, malformed terminal data, or clean EOF before completion
fail rather than returning partial success. SSE comments/keepalives do not
count as model progress; consumed provider JSON events do, including role-
only, reasoning, and tool events.

Map `stop` to `Stop`, `tool_calls` to `ToolCalls`, `length` to `Truncated`,
and `content_filter` to `Filtered`. Other values become `Other(raw)`; a missing
finish reason fails terminal validation. Only `Stop` is a natural completion.
Parse dispatchable tools only for `ToolCalls`; require a
nonempty collection, unique nonblank ids, names, and valid raw JSON arguments.
Terminal/truncated/filtered responses suppress tool payloads before strict
tool decoding. The adapter returns one response and never executes tools.

Keep `ModelResponse.provider` as `openrouter`, its catalog id as selected,
and `model_id` as the catalog wire id. Validate returned model identity against
that exact wire id; an unexpected alias or substituted model is an error.
Record the effective thinking level from the entry. The selected fixed
upstream remains provider metadata, not a different `ProviderKind`.

Emit nonempty primary-choice assistant deltas exactly, preserving terminal
parity, and separate exposed reasoning deltas as defined in
[OpenRouter replay](openrouter-replay.md). Never emit signatures, encrypted
data, raw tools, usage, or provider payloads as public text events.

Before progress, classify HTTP 429 as rate-limited; 408/409/425/5xx and
transport failures as transient; 400/401/402/403 and other terminal statuses
as provider failures. A typed gateway error object in an HTTP-success stream
uses its numeric status-equivalent code with the same policy. Unknown codes
are terminal. Do not classify by message substrings. All failures after
provider progress become `Interrupted`; no automatic fallback follows them
in the new router. Local typed serialization/configuration failures use the
tracked internal error contract. Error messages must not contain credentials,
complete prompts, or retained private reasoning payloads.

## Verification And Live Coverage

Unit tests use `unimock` HTTP/auth collaborators and external `_tests_`
modules. Cover every catalog/profile combination, construction failures,
controls, rejected modalities, malformed dispatchable tools, terminal tool
suppression, structured validation, status/payload errors, stream interruption,
final identity, token/cost mapping, and buffered/event parity. Cover generation
id serde/defaults, missing/repeated/conflicting ids, response-only plain-text
retention, wrappers/router, logger copies, and response checkpoints on text
and tool rounds. Replay and accounting acceptance tests are specified in their
linked protocols.

Add credential-free construction to `cargo xtask smoke-test`. Add OpenRouter
to `xtask/tests/live_models.rs`, its provider registry/guards, and
`.github/workflows/live-models.yml` with `OPENROUTER_API_KEY`. Keep existing
same-repository secret protections. Ignored `openrouter_catalog` runs every
entry through `DynModel` and `ToolCallingRuntime` with event observation,
identity/thinking checks, token usage, and provider-cost assertions.

The OpenRouter job also performs a two-turn tool continuation with high Sonnet
and validates retained reasoning details without printing them. This probe
uses automatic tools plus a prompt directing the test tool; it must observe
the tool call and complete from its result. It is an explicit exception to
the general connectivity suite's exclusion of replay/pricing tests, recorded
in the live-test protocol when implemented.

Billable invocation, outside `check` and credential-free smoke tests:

```sh
LIVE_MODEL_API_KEY="$OPENROUTER_API_KEY" cargo test --locked -p xtask \
  --test live_models openrouter_catalog -- --ignored --exact --nocapture
```

Live credentials are an implementation acceptance dependency. Missing
credentials or unavailable upstream models must be reported as incomplete
live validation; deterministic passing tests do not substitute for it.
