# Add Video Generation Support

## Summary

Add a provider-agnostic video generation boundary to `ai-interface` and
implement it for OpenAI Sora and Google Veo, mirroring the established image
generation architecture. The normative behavior is defined by the
[Video Generation Protocol](../docs/protocol/video-generation.md).

## Scope

This change includes shared typed DTOs and errors, a deterministic mock,
`ModelFeature::VideoGeneration`, authenticated binary HTTP downloads, OpenAI
and Google job submission/polling/download adapters, current catalog entries,
credential-free tests, ignored catalog-driven live tests, and documentation.

One optional first-frame image, portable 4/8-second durations, landscape or
portrait 720p output, and exactly one MP4 are in scope. Webhooks, progress
callbacks, cancellation, durable jobs, multiple outputs, video edits or
extensions, last-frame/reference-image controls, provider-only durations or
resolutions, and pricing are out of scope.

## Milestone 1: Protocol And Plan

Define the contract before code so provider and consumer behavior is explicit.

- [x] Verify the current OpenAI Sora and Google Veo official API contracts.
- [x] Define request/response DTOs, portable controls, polling, result, usage,
      and complete error semantics.
- [x] Register this plan under Active in `plans/README.md`.
- [x] Link the protocol from the workspace README.

## Milestone 2: Shared Boundary And HTTP Transport

Consumers can compile against and deterministically mock the boundary, and
providers can fetch authenticated binary assets.

- [x] Add failing DTO, serde, default, error, mock, and routing feature tests.
- [x] Implement `VideoGenerator`, its dyn alias, DTOs, tracked error contract,
      and deterministic `MockVideoGenerator`.
- [x] Add an authenticated binary-response path to `json-http` behind the
      existing transport/client traits, with request/auth/timeout tests.
- [x] Update `ai-interface` and `json-http` READMEs.
- [x] Run targeted formatting, Clippy, and tests.

## Milestone 3: OpenAI Sora Adapter

Callers can generate one completed MP4 through `OpenAiVideoGenerator`.

- [x] Add failing JSON/multipart request mapping and validation tests.
- [x] Add failing job status, failure classification, polling, timeout,
      authenticated download, and empty-video tests.
- [x] Implement trait-backed submission, polling, and download with injected
      auth and polling-runtime collaborators.
- [x] Add `sora-2` catalog metadata and tests.
- [x] Update the OpenAI crate README and construction smoke test.

## Milestone 4: Google Veo Adapter

The same trait works through Google's long-running operation API.

- [x] Add failing request mapping and first-frame validation tests.
- [x] Add failing operation/error/response, polling, timeout, authenticated
      download, and empty-video tests.
- [x] Implement trait-backed submission, operation polling, and download with
      injected auth and polling-runtime collaborators.
- [x] Add `veo-3.1-generate-preview` catalog metadata and tests.
- [x] Update the Google crate README and construction smoke test.

## Milestone 5: Live Coverage And Documentation

Catalog registration automatically produces credentialed coverage.

- [x] Add a centralized ignored `live_videos` suite and trusted-branch Actions
      workflow covering every video-capable provider catalog.
- [x] Validate the shortest provider-neutral request and MP4 signature without
      persisting generated assets or logging credentials.
- [x] Update root, xtask, protocol, and relevant crate documentation.
- [x] Mark the protocol Implemented and move this plan to Completed after all
      implementation and verification items pass.

## Milestone 6: Verification, Commit, Push, And Review

- [x] Run targeted tests, `cargo fmt --all -- --check`, strict workspace
      Clippy, all workspace tests, file-length lint, and smoke tests.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for scope, docs, public API, tests, and
      untracked files.
- [x] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters and a descriptive body, and push the current branch
      without renaming it.
- [x] Run `cargo xtask review` after the push against `origin/main`.
- [x] Do not auto-fix review findings; report each with severity, context,
      impact, lettered options, and the recommended option.
