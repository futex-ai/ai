# Add Image Generation Support

## Summary

Add a provider-agnostic image generation boundary to `ai-interface` and
implement it for OpenAI and Google. The design follows the existing
`AudioTranscriber` modality boundary: interface-owned DTOs and typed errors, a
unimock-enabled trait and dynamic alias, a deterministic built-in mock, and
provider implementations beside each provider's chat model.

The normative behavior is defined by the
[Image Generation Protocol](../docs/protocol/image-generation.md).

The downstream consumer is juno's Platform API. It exposes image generation as
an OpenAI-style built-in tool inside agentic bursts and pins this workspace by
git revision.

## Verified Provider Baselines

The implementation baseline was verified against official provider references
on 2026-08-11:

- OpenAI remains `gpt-image-2` through the Image API generation and edit
  endpoints.
- Google's current GA, recommended balanced image model is
  `gemini-3.1-flash-image`. This supersedes the handoff draft's
  `gemini-2.5-flash-image` baseline. The implementation uses the current
  `generateContent` compatibility endpoint and skips interim thought images.

## Consumer Contract

- Content-policy refusals are typed separately from transport/provider errors.
- Each trait call returns exactly one image; there is no `n` parameter.
- Responses contain decoded bytes and a MIME type. Storage, retention, and
  wire-level base64 encoding belong to the consumer.
- Aspect and quality are provider-agnostic, best-effort controls rather than
  exact pixel contracts.
- Non-empty input images select edit mode.
- Provider-reported tokens populate `ModelUsage`; this workspace adds no image
  pricing.
- Image-capable catalog entries advertise `ModelFeature::ImageGeneration`.
- Retry classification follows the transcription boundary's rate-limited,
  transient-provider, and terminal-provider conventions.

## Scope

This change includes the protocol, shared trait and DTOs, deterministic mock,
routing feature, OpenAI generation/edit support, Google generation/edit
support, catalog entries, credential-free tests, ignored live smoke tests, and
README updates.

Streaming, partial or multiple images, exact output dimensions, caller-selected
formats/compression/backgrounds, masks, moderation controls, pricing wrappers,
retry/concurrency wrappers, and credentialed CI are out of scope.

## Milestone 1: Protocol And Plan

Define the contract before code. At the end of this milestone, implementation
requires no provider or consumer guesswork.

- [x] Register this plan under Active in `plans/README.md`.
- [x] Define request/response DTOs, one-image semantics, edit selection,
      normalized controls, usage behavior, and the full error taxonomy.
- [x] Verify and document the current OpenAI model, endpoints, request options,
      output, usage, and content-policy mapping.
- [x] Verify and document the current Google model, request/response shape,
      aspect mapping, thought-image handling, usage, and safety mapping.
- [x] Cross-link the protocol from the workspace README.

## Milestone 2: `ai-interface` Contract

At the end of this milestone, consumers can depend on and mock the boundary
without either provider crate.

- [x] Add failing serde and construction tests for all request/response DTOs,
      enum defaults, and snake-case serialization.
- [x] Add failing display/helper tests for `ImageGenerationError`.
- [x] Implement the DTOs, typed error, result alias, unimock-enabled
      `ImageGenerator`, and `DynImageGenerator`.
- [x] Add failing tests and implement `MockImageGenerator` with a configurable,
      checked valid PNG response and blank-prompt rejection.
- [x] Add failing config/display/serde tests and implement
      `ModelFeature::ImageGeneration` across exhaustive matches.
- [x] Export the public API and update `crates/ai-interface/README.md`.
- [x] Run interface/core formatting, Clippy, and tests.

## Milestone 3: OpenAI Implementation

At the end of this milestone, callers can generate or edit exactly one image
through `OpenAiImageGenerator`.

- [x] Add failing generation JSON and edit multipart-selection tests, including
      aspect/quality mappings and local validation.
- [x] Add failing response tests for base64 decoding, MIME inference,
      `revised_prompt`, usage, malformed base64, and missing images.
- [x] Add failing classification tests for rate limits, transient statuses and
      transport failures, content-policy codes, and other provider failures.
- [x] Implement explicit construction, endpoint/timeout overrides, generation,
      and multipart edit calls in focused modules below the file cap.
- [x] Add failing catalog tests and a current `gpt-image-2` entry advertising
      image generation.
- [x] Add an ignored, environment-gated live smoke test.
- [x] Update `crates/ai-models-openai/README.md` and run provider checks.

## Milestone 4: Google Implementation

At the end of this milestone, the same trait is implemented through Gemini's
structurally different JSON API.

- [x] Add failing request tests for prompt/input image parts, image-only
      response modality, aspect mapping, and ignored quality.
- [x] Add failing response tests for final inline image decoding, MIME type,
      missing images, malformed base64, and normalized usage.
- [x] Add failing tests proving interim thought images are skipped.
- [x] Add failing safety tests for prompt blocks and safety/policy finish
      reasons, retaining provider messages.
- [x] Implement explicit construction and endpoint override through injected
      JSON HTTP/auth collaborators.
- [x] Add failing catalog tests and a current `gemini-3.1-flash-image` entry
      advertising image generation.
- [x] Add an ignored, environment-gated live smoke test.
- [x] Update `crates/ai-models-google/README.md` and run provider checks.

## Milestone 5: Workspace Verification And Handoff

At the end of this milestone, the feature is documented, fully verified, and
ready for the downstream revision bump.

- [x] Confirm no retry/pricing/concurrency wrappers were added and existing
      chat model behavior is unchanged.
- [x] Run all targeted tests, full formatting, Clippy, workspace tests,
      credential-free smoke tests, and the Rust file-length lint.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for scope, docs, public API, tests, and
      untracked files.
- [x] Mark the protocol Implemented and move this plan to Completed only after
      every implementation and verification task is complete.
- [x] Run `git add -A` so every new source, test, protocol, plan, README, and
      lockfile change is tracked.
- [x] Commit with a Conventional Commit title no longer than 50 characters and
      a descriptive body.
- [x] Push the current branch without renaming it.
- [x] Run `cargo xtask review` after the push against `origin/main`.
- [x] Do not auto-fix review findings; report each with a number, severity,
      context, impact, lettered options, and a recommended option.
- [x] Document the post-merge requirement to record the merged revision in the
      eventual PR description so juno can update its `futex-ai/ai` git pins.

## Review And Post-Merge Handoff

The mandatory AI review completed after the implementation commit was pushed.
It reported three P2 findings: reject empty decoded Google image payloads,
adopt the workspace `InternalError` contract at the shared boundary, and inject
a trait-backed OpenAI HTTP transport. Repository policy requires these findings
to be reported for a follow-up decision rather than fixed automatically after
review.

The pushed implementation revision at review time was
`f1b972fa2cbd27f677d4649bd31d1d3cbbc3667c`. This is not the eventual merged
revision. The PR description must record that merged revision after landing so
juno can update its `futex-ai/ai` git pins.
