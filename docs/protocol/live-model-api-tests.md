# Live Model API Test Protocol

## Purpose

Continuously verify that every real chat-provider adapter and every
chat-capable exported catalog variant can complete a request through the
generic tool-calling runtime and its upstream production API.

## Scope

The credentialed suite covers Anthropic, DeepSeek, Google Gemini, Kimi,
MiniMax, OpenAI, QwenCloud, and xAI. Each provider test obtains its models from
that crate's `known_models()` function, so new chat-capable catalog variants
enter the live test automatically. Logical variants that share an upstream
model id still run separately because they exercise different thinking
controls. Entries advertising `ModelFeature::ImageGeneration` or
`ModelFeature::VideoGeneration` are routed through `ImageGenerator` or
`VideoGenerator`, not `Model`, and are therefore outside this chat connectivity
suite.

Audio transcription, image or video generation, streaming, provider-built
tools, multimodal input, multi-turn tool replay, pricing, and provider features
outside the chat model catalog are not part of this connectivity suite.
Deterministic transport tests in the provider crates remain responsible for
detailed wire behavior.

Credentialed image generation is specified separately by the implemented
[live image API test protocol](live-image-api-tests.md). Keeping the suites
separate preserves their distinct traits, success contracts, costs, and CI
controls.

Credentialed video generation is specified separately by the implemented
[live video API test protocol](live-video-api-tests.md) for the same reason.

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
2. Selects every catalog entry that advertises neither image nor video
   generation, then
   constructs it with `ReqwestJsonHttpClient`, the production chat adapter,
   explicit provider authentication, and catalog thinking metadata.
3. Erases the concrete adapter behind `DynModel` and applies the standard
   transient retry wrapper.
4. Injects that model into `ToolCallingRuntime`, sends one text-only turn with
   no application tools or schema, and captures the normalized response through
   the runtime's provider-neutral response-checkpoint boundary.
5. Applies one portable control set to every provider: no tool calls, a
   ten-minute adapter-call deadline, and `PreferDeferred`. XAI resolves that
   preference to its native deferred lifecycle; every other adapter falls back
   to its ordinary synchronous lifecycle without a provider branch in the
   runner.
6. Continues after a model failure and reports all failures for that provider
   together.

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
- retain the catalog id, upstream model id, and thinking level selected by the
  catalog entry;
- finish naturally with non-empty text containing the probe marker;
- return no tool calls when none were offered; and
- report a non-zero total token count.

A provider job fails if any catalog entry violates this contract. For an
eligible event, missing credentials also fail explicitly and must never be
treated as skipped or successful coverage.

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
