# Live Model API Test Protocol

## Purpose

Continuously verify that every real chat-provider adapter and every listed
effort of every chat-capable exported model spec can complete a request
through the generic tool-calling runtime and its upstream production API.

## Scope

The credentialed suite covers Anthropic, DeepSeek, Google Gemini, Kimi,
MiniMax, OpenAI, QwenCloud, and xAI. Each provider test obtains its specs from
that crate's `known_models()` function. It runs one probe for each effort
that a chat-capable spec lists, with the default effort first, so new specs
and efforts enter the live test automatically. Probes that share a spec
still run separately because each effort exercises different thinking
controls. Specs advertising `ModelFeature::ImageGeneration`,
`ModelFeature::VideoGeneration`, or `ModelFeature::Judgment` are routed through
their specialized provider-neutral traits, not `Model`, and are therefore
outside this chat connectivity suite.

Audio transcription, image or video generation, provider-built tools,
multimodal input, multi-turn tool replay, pricing, and provider features
outside the chat model catalog are not part of this connectivity suite. All
eight providers implement `Model::stream`, the one entry point of the
[model stream protocol](model-stream.md). Each live provider job starts
with an explicit synchronous event probe, then observes events while exercising
the full catalog with `PreferDeferred`. That preference falls back to the
synchronous lifecycle for Anthropic, DeepSeek, Google, Kimi, MiniMax, OpenAI,
and QwenCloud. For xAI it selects the submit-and-poll lifecycle, which emits
every event of the call after the provider result arrives. Deterministic
transport tests in the provider crates remain responsible for detailed event
ordering and wire behavior.

Credentialed image generation is specified separately by the implemented
[live image API test protocol](live-image-api-tests.md). Keeping the suites
separate preserves their distinct traits, success contracts, costs, and CI
controls.

No live suite covers video generation, because no catalog lists a video
model.

Credentialed judgment evaluation is specified separately by the implemented
[live judgment API test protocol](live-judgment-api-tests.md).

MiniMax is the one additional tool-choice compatibility probe: before its
catalog connectivity loop, the suite calls `MiniMax-M3` with a real function
definition and strict `ModelToolChoice::Required`, then requires a matching
tool call in the provider response. This preserves the live-verified required
mapping even though the public MiniMax parameter reference lists only `auto`
and `none`.

## Execution

`xtask/tests/live_models.rs` owns the ignored integration tests. Every provider
test:

1. Requires a non-empty `LIVE_MODEL_API_KEY`.
2. Selects every spec that advertises neither image generation, video
   generation, nor judgment, and expands it into one probe for each listed
   effort, with the default effort first. It constructs the production chat
   adapter from the spec with `ReqwestJsonHttpClient` and explicit provider
   authentication, and it sets the effort of the probe in the request
   controls.
   Each probe uses the production adapter lifecycle without a live-test-only
   branch.
3. Erases the concrete adapter behind `DynModel` and applies the standard
   transient retry wrapper.
4. Wraps the dynamic model in a test-only `Model` whose `stream` forwards
   the events of the inner model's `stream` and records them in order.
   `ToolCallingRuntime` folds the same events through the provided
   `complete` and captures the normalized response through its
   provider-neutral response-checkpoint boundary.
5. Runs one synchronous probe against the provider's first chat spec at its
   default effort and requires text-event parity with its response.
6. Runs every probe with no tool calls, a ten-minute adapter-call
   deadline, and `PreferDeferred`. XAI resolves that preference to its native
   deferred lifecycle; every other adapter falls back to its ordinary
   synchronous lifecycle. Every probe must keep text-event parity.
7. Retries the synchronous probe or a catalog probe exactly once when a
   successful response fails only because it omitted the probe marker and its
   event validation is otherwise clean, apart from a possible terminal-parity
   failure against that same response. Natural-language replies can be
   non-deterministic even under an exact prompt, so the second result replaces
   the first. Request errors and identity, finish, tool, usage, or other event
   failures are not retried by this rule. It can add at most one billable call
   per probe.
8. Continues after a probe failure and reports all failures for that
   provider together.

The MiniMax provider test first runs its strict required-tool probe. A failure
there aborts that provider job before the ordinary catalog loop.

Provider-specific matching is restricted to the test composition registry that
loads catalogs, authentication, and concrete adapters. Once construction is
complete, the execution path depends only on `DynModel`, `ModelCallControls`,
and `ToolCallingRuntime`.

The normal workspace test and `cargo xtask check` commands compile the suite,
run its credential-free coverage guards, and leave the eight credentialed
tests ignored.

## Success Contract

Each live completion must:

- complete one generic runtime step;
- return the expected normalized provider id;
- record the text id of the spec, the provider model id, and the effort of
  the probe;
- finish naturally with non-empty text containing the probe marker;
- return no tool calls when none were offered; and
- report a non-zero total token count.

Every probe, synchronous or deferred, must also emit at least one nonempty
text delta. Text-event parity means that the text parts of those deltas,
each joined from its deltas and then joined in part position order, exactly
reproduce the response text. Text and reasoning deltas follow the
[delta rules of the model stream protocol](model-stream.md#model-events).
A direct provider probe must not emit a fallback event.
Each successful stream emits exactly one usage event, then exactly one finish
event. A request with a response schema must emit text deltas with the same
parity.

A marker-only miss receives one bounded retry because model replies can vary;
the second result must satisfy the complete contract. The retry does not mask
identity, finish-reason, tool, usage, request, or non-parity event failures and
does not alter the separate transient transport retry policy. Its maximum cost
is one additional billable call for each synchronous probe or catalog probe.
A provider job fails if any final probe result violates this contract. For an
eligible event, missing credentials also fail explicitly and must never be
treated as skipped or successful coverage.

Credential-free tests enforce that all eight registered providers receive the
synchronous parity probe and the parity expectation for their catalog probes,
including xAI with its deferred lifecycle. They also enforce that the test
model records the events of the inner stream, and that the parity validator
rejects incomplete observations. They check schema text, usage, and finish
events without adding a paid probe. They do not send provider requests.

## CI And Credentials

`.github/workflows/live-models.yml` runs for pull requests targeting `main`, on
the default branch on a daily schedule, and by manual dispatch from the default
branch. It uses one matrix job per provider, keeps matrix failures independent,
and limits parallelism to reduce rate pressure. A newer revision of the same
pull request cancels its superseded live run. The workflow has read-only
repository permissions and exposes only the current provider's secret, only to
the credential check and test steps.

The required repository Actions secrets are:

| Provider | Secret |
| --- | --- |
| Anthropic | `ANTHROPIC_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Google | `GOOGLE_API_KEY` |
| Kimi | `KIMI_API_KEY` |
| MiniMax | `MINIMAX_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| QwenCloud | `QWEN_API_KEY` |
| xAI | `XAI_API_KEY` |

Credentialed jobs run only for same-repository pull requests not authored by
Dependabot. GitHub withholds repository Actions secrets from forked and
Dependabot pull requests, so those jobs are skipped before checkout or secret
access. The workflow must not use `pull_request_target` to execute pull-request
code because that would give untrusted code a privileged secret context.

## Local Invocation

Live runs are billable. Run one provider catalog explicitly:

```sh
LIVE_MODEL_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask \
  --test live_models openai_catalog -- --ignored --exact --nocapture
```
