# Provider Call Controls

## Goal

Make `ai_interface::ModelRequest` the provider-neutral source of generation
and execution intent. Each `ai-models-*` adapter remains the sole owner of its
wire request and lifecycle, including XAI deferred completion. Consumers such
as Firna choose models, routing, limits, and deadlines without inspecting
provider names, endpoints, or native JSON fields.

Provider identity remains at Firna's configuration/composition boundary so it
can select an adapter and supply credentials. Firna feature, evaluation, and
execution-policy code should receive `dyn Model` plus `ModelRequest` and remain
provider-neutral.

The reviewed handoff is retained at
`.context/ai-repo-provider-call-controls-plan.md`. That working copy is not a
tracked project artifact.

## Design Decisions

- Add defaulted generation and execution controls to `ModelRequest`; do not
  change the `Model` trait.
- Keep generation controls portable: temperature, top-p, maximum output
  tokens, ordered stop sequences, and typed tool choice.
- Model execution separately with a total call timeout and a completion
  preference: synchronous-only, prefer deferred, or require deferred.
- `prefer deferred` uses a provider's synchronous lifecycle when deferred
  completion is unavailable. `require deferred` returns a typed unsupported-
  control error when unavailable. Consumers never branch on provider identity.
- Default controls preserve existing request behavior except for the reviewed
  compatibility fixes: blank system omission and Google full JSON Schema.
- Explicit controls are either mapped, deliberately ignored where a provider
  fixes the value, or rejected with a typed unsupported-control error. They are
  never inserted by post-serialization JSON mutation.
- A timeout is a total adapter-call deadline. Synchronous adapters apply it to
  their one HTTP request. XAI applies it across submission and all polls, with
  a smaller provider-owned per-poll timeout.

## Milestone 1: Protocol Contract

Define the public and provider behavior before changing implementation.

- [x] Document shared generation controls, execution controls, defaults,
      errors, and ownership.
- [x] Document supported, fixed, and unsupported controls for OpenAI,
      Anthropic, Google, Kimi, QwenCloud, DeepSeek, MiniMax, and XAI.
- [x] Document blank-system behavior and Google schema field ownership.
- [x] Document XAI deferred submission, same-id polling, retry, and timeout
      semantics.
- [x] Record Firna's provider-neutral migration contract.

## Milestone 2: Shared Interface

Introduce the portable seam while retaining default behavior.

- [x] Add typed generation controls and tool choice to `ai-interface`.
- [x] Add typed execution controls and completion preference.
- [x] Add typed unsupported-control errors.
- [x] Update every `ModelRequest` constructor and public example.
- [x] Prove mocks and retry, concurrency, pricing, fallback, structured-output,
      and tool-calling paths preserve the controls unchanged.
- [x] Keep all shared and transport code free of provider logic.

## Milestone 3: Provider Mappers

Make each adapter own its final native request.

- [x] Implement and test OpenAI controls and reasoning omissions.
- [x] Implement and test Anthropic controls, thinking restrictions, and native
      output bound.
- [x] Implement and test Google controls, tool configuration, and full
      function JSON Schema serialization.
- [x] Implement and test Kimi fixed sampling, supported controls, and blank
      system omission.
- [x] Implement and test Qwen thinking restrictions, supported controls, and
      blank system omission.
- [x] Implement and test DeepSeek V4 controls, thinking restrictions, exact
      endpoint, and blank system omission.
- [x] Implement and test MiniMax supported controls and blank system omission.
- [x] Implement and test XAI controls and blank system omission.
- [x] Inspect final requests at mocked `JsonHttpTransport` boundaries.

## Milestone 4: XAI Deferred Lifecycle

Move deferred execution behind the XAI adapter.

- [x] Add deferred submission and typed accepted-id parsing.
- [x] Add an injected clock/sleeper seam and same-id polling.
- [x] Retry retrieval through pending, rate limiting, transient transport
      failures, and server errors without resubmitting.
- [x] Enforce the total deadline and bounded per-poll timeout.
- [x] Validate request IDs before path construction and preserve auth on every
      poll.
- [x] Test success, pending, retries, terminal errors, malformed IDs, timeout,
      and exactly one submission.

## Milestone 5: Documentation and Verification

Finish with a fully checked, pin-ready repository revision.

- [x] Update affected crate READMEs and the root README.
- [x] Run all targeted crate tests and credential-free smoke checks.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run workspace Clippy and all workspace tests.
- [x] Run `cargo xtask check` and fix every failure.
- [x] Review the complete diff against `origin/main`.

## Milestone 6: Commit, Push, and Review

Publish the verified work and collect review findings without changing them.

- [x] Move this plan from Active to Completed in `plans/README.md`.
- [x] Stage every changed and newly created tracked file with `git add -A`.
- [x] Commit with a Conventional Commit title of at most 50 characters and a
      descriptive body.
- [x] Push the current branch without renaming it.
- [x] Run `cargo xtask review` after the push against `origin/main`.
- [x] Report numbered review findings with severity, context, impact, lettered
      solution options, and a recommended option; do not automatically fix
      review findings.
- [x] Report the commit SHA, tests, public migration, and intentionally
      unsupported controls for Firna.

The post-push review reported one P2 finding: omission checks treat only an
empty system prompt as blank, while the protocol wording also covers
whitespace-only prompts. The reviewed implementation was left unchanged so
the finding can be resolved through the repository's normal review decision.
