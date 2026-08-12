# Live Model API Test Protocol

## Purpose

Continuously verify that every real provider adapter and every exported catalog
variant can complete a request through its upstream production API.

## Scope

The credentialed suite covers Anthropic, DeepSeek, Google Gemini, Kimi,
MiniMax, OpenAI, QwenCloud, and xAI. Each provider test obtains its models from
that crate's `known_models()` function, so new catalog variants enter the live
test automatically. Logical variants that share an upstream model id still run
separately because they exercise different thinking controls.

Audio transcription, streaming, provider-built tools, multimodal input,
multi-turn tool replay, pricing, and provider features outside the model
catalog are not part of this connectivity suite. Deterministic transport tests
in the provider crates remain responsible for detailed wire behavior.

## Execution

`xtask/tests/live_models.rs` owns the ignored integration tests. Every provider
test:

1. Requires a non-empty `LIVE_MODEL_API_KEY`.
2. Constructs every catalog entry with `ReqwestJsonHttpClient`, the production
   adapter, explicit provider authentication, and catalog thinking metadata.
3. Sends a minimal text-only completion request with no tools or schema.
4. Applies the standard transient retry wrapper.
5. Continues after a model failure and reports all failures for that provider
   together.

The normal workspace test and `cargo xtask check` commands compile the suite,
run its credential-free coverage guards, and leave the eight credentialed
tests ignored.

## Success Contract

Each live completion must:

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
