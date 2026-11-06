# Development

This page describes the credentialed live checks, the CI workflows, and the
steps that the downstream host Firna takes to adopt the model stream. The
[workspace README](../README.md) gives the short getting-started commands.
The protocol docs under [`protocol/`](protocol/) hold the requirements of
each live suite.

## Live model checks

Credentialed model checks live in `xtask/tests/live_models.rs`. They construct
provider adapters only at the composition boundary. They then call each
chat-capable model spec once at its default and once for each other effort
that the spec lists, through the same `DynModel` and `ToolCallingRuntime`
path with portable controls. The default call of a spec without a default
effort sets no effort. Each provider first proves synchronous text
parity on its first chat spec at its default effort, and every catalog call,
also each xAI deferred call, keeps that parity. Parity means that the text
parts of the deltas, joined in part position order, equal the response
text. Run the credential-free catalog, bridge, validation, and workflow
guards with:

```sh
cargo test --locked -p xtask --test live_models
```

To test one provider against the real API, set `LIVE_MODEL_API_KEY` and run the
corresponding ignored test, for example:

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
its provider job. The [live model API test protocol](protocol/live-model-api-tests.md)
defines the probes and their success contract.

## Live image checks

Credentialed image checks live in `xtask/tests/live_images/mod.rs`. They select
every Google and OpenAI catalog spec advertising `ImageGeneration`, construct
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

## Live judgment checks

Credentialed judgment checks live in `xtask/tests/live_judgments/mod.rs`. They
select every TypeSafe catalog spec advertising `Judgment`, construct the
production adapter behind `DynJudgmentModel`, and evaluate one short support
message against condition, choice, and a fully described score rubric. Run the
credential-free guards or the billable TypeSafe catalog with:

```sh
cargo test --locked -p xtask --test live_judgments
LIVE_JUDGMENT_API_KEY="$TYPESAFE_API_KEY" cargo test --locked -p xtask \
  --test live_judgments catalog_tests::typesafe_judgment_catalog \
  -- --ignored --exact --nocapture
```

The `Live judgment APIs` workflow runs the TypeSafe catalog sequentially for
trusted same-repository pull requests, daily, and on manual dispatch after the
repository variable `LIVE_JUDGMENT_API_TESTS_ENABLED` is set to `true`. Enable
it only after provisioning `TYPESAFE_API_KEY`; once enabled, a missing secret
fails the job. The workflow may make up to three billable attempts per catalog
model after transient failures.

## Live video checks

No live suite covers video generation, because no catalog lists a video model.

## CI

GitHub Actions runs the same Rust verification expected locally on pull requests
and pushes to `main`: formatting, Clippy, tests, Rust file-length lint,
credential-free smoke tests, and `cargo xtask check`. Limiting push-triggered CI
to `main` prevents an open pull request from running the same commit once for
the branch push and again for the pull-request event. The separate `Live model
APIs` workflow makes billable calls through the production adapters for the
default and each listed effort of every chat-capable catalog spec through
the generic tool-calling runtime on eligible pull requests as well as its
daily schedule and manual dispatch. The separate
`Live image APIs` workflow exercises every Google and OpenAI image-capable
catalog spec through `DynImageGenerator` with image-specific validation and
sequential provider jobs. The `Live judgment APIs`
workflow exercises every TypeSafe Jev catalog spec through `DynJudgmentModel`
with `TYPESAFE_API_KEY`, shared answer validation, and sequential retries once
`LIVE_JUDGMENT_API_TESTS_ENABLED` opts the repository into those billable
checks. Forked and Dependabot pull requests skip all credentialed workflows
because GitHub does not provide them repository Actions secrets. The ordinary
credential-free suite still enforces the live model registry without making
provider calls.

## Firna adoption

Firna is a downstream host of these crates. This workspace does not change
Firna. Before Firna pins a revision with the model stream, it must take
these steps:

1. Implement `JsonHttpTransport::execute_sse` in its
   `BoundedModelHttpTransport`, and keep its bounded rewrite of the request
   body. Until then, Firna compiles through the default method, which
   returns `SseUnsupported`, but every model call fails at runtime.
2. Update its URL matching. A pattern for Google's `:generateContent` does
   not match the `:streamGenerateContent` endpoint that the Google adapter
   calls.
3. Read `error.core().kind` to handle a stream that failed after progress
   with the interrupted kind.
4. Treat its per-call `total_timeout` override as the overall deadline of
   the model stream. The override does not change the idle timeout.
5. Implement `Model::stream` in each of its own `Model` implementations.
   The provided `complete` folds the events, so callers that need only the
   response keep calling `complete`.
6. Read `Model::stream` at its platform event boundary for live output.
   Forward text, reasoning, and fallback events in order. When a `Fallback`
   event arrives, discard the output of the failed attempt: every event
   since the start of the stream or since the previous fallback event.

The [model stream protocol](protocol/model-stream.md) gives the events and
their order. The
[provider call controls protocol](protocol/provider-call-controls.md#firna-migration)
gives the Firna steps for the call controls.
