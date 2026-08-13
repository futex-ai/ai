# Credentialed Image API CI

## Summary

Add a dedicated credentialed CI suite that generates one real image through
every image-capable catalog entry. The suite will initially cover the existing
Google and OpenAI adapters, discover catalog additions automatically, validate
normalized image responses, and retain the repository's trusted-secret
boundary while keeping calls and cost deliberately small.

The normative target behavior is defined by the
[Live Image API Test Protocol](../docs/protocol/live-image-api-tests.md).

## Scope

This change includes a centralized ignored `xtask` integration suite,
credential-free registry and workflow guards, typed transient retry behavior,
a Google/OpenAI CI matrix, local invocation documentation, and migration of the
provider-local ignored smoke tests into the centralized suite.

Adding image providers or models, live image editing, visual-quality
evaluation, multiple outputs, artifact retention, production retry wrappers,
pricing, and changes to the shared `ImageGenerator` contract are out of scope.

## Milestone 1: Protocol And Plan

Define the complete connectivity, security, cost, and success contract before
implementation. At the end of this milestone, implementation requires no
policy or coverage decisions.

- [x] Define catalog-driven coverage and the initial Google/OpenAI provider
      set.
- [x] Define the exact low-cost generation request, typed retry boundary, and
      normalized response assertions.
- [x] Define trusted pull-request, schedule, manual-dispatch, credential, cost,
      concurrency, and generated-data handling requirements.
- [x] Keep chat connectivity and image connectivity as separate suites and
      cross-link their protocols.
- [x] Register this plan under Active in `plans/README.md`.

## Milestone 2: Central Live-Image Test Harness

Build the ignored integration suite and its credential-free seams. At the end
of this milestone, both production image adapters can be exercised through one
catalog-driven runner without making live calls during ordinary tests.

- [ ] Add failing registry tests proving every catalog entry advertising
      `ImageGeneration` belongs to a registered live-image provider, every
      registered provider has at least one image entry, and non-image entries
      are excluded.
- [ ] Implement a focused `LiveImageProvider` registry for provider identity,
      catalog access, workflow test/secret names, authentication, and
      construction of `DynImageGenerator` from the catalog upstream model id.
- [ ] Add failing runner tests for strict non-empty
      `LIVE_IMAGE_API_KEY` handling, sequential catalog iteration, aggregated
      failures, and provider-neutral dynamic dispatch.
- [ ] Add failing retry tests proving only `RateLimited` and
      `TransientProvider` receive bounded retries and all terminal error classes
      stop immediately.
- [ ] Add failing validation tests for provider/model identity, non-empty
      payloads, supported MIME types, and PNG/JPEG/WebP signature agreement.
- [ ] Implement the safe square, low-quality, one-image probe and ensure logs
      contain identifiers and concise failures but never image bytes or
      credentials.
- [ ] Add ignored `google_image_catalog` and `openai_image_catalog` tests that
      call the production adapters through the shared runner.
- [ ] Remove the superseded provider-local ignored live smoke tests while
      retaining their deterministic transport and response tests.
- [ ] Keep source and test modules cohesive and every changed Rust file below
      the 300-line cap.

## Milestone 3: Credentialed CI Workflow

Wire the suite into GitHub Actions with explicit security and spend limits. At
the end of this milestone, eligible CI events make one real generation call per
image catalog model and fail visibly on missing coverage or credentials.

- [ ] Add failing workflow guard tests for every registered provider test and
      secret, trusted pull-request gating, default-branch schedule/manual
      gating, read-only permissions, and the exact ignored-test command.
- [ ] Create `.github/workflows/live-images.yml` with independent Google and
      OpenAI matrix entries, `fail-fast: false`, `max-parallel: 1`, bounded job
      timeout, and superseded-run cancellation.
- [ ] Expose only the current provider secret as `LIVE_IMAGE_API_KEY`, require
      it before checkout/test execution, and fail rather than silently skip
      eligible coverage.
- [ ] Run only for same-repository pull requests not authored by Dependabot,
      daily on the default branch, and manual dispatch without using
      `pull_request_target`.
- [ ] Confirm the workflow never uploads generated image artifacts and that
      ordinary CI, `cargo xtask check`, and `cargo xtask smoke-test` remain
      credential-free.

## Milestone 4: Documentation And Focused Verification

Document the implemented boundary and prove the harness without spending API
credits during normal development. At the end of this milestone, maintainers
can discover, run, and safely extend live image coverage.

- [ ] Mark the live-image protocol implemented and align the image-generation
      and live-model protocols with the new suite.
- [ ] Update the workspace and `xtask` READMEs with CI cadence, provider/model
      discovery, billable local commands, exclusions, and key code locations.
- [ ] Add extension guidance requiring each future image provider to register
      its catalog, adapter construction, CI test, and secret mapping.
- [ ] Run the credential-free `live_images` integration tests and focused
      Google/OpenAI image adapter tests with a 100% pass rate.
- [ ] Run `cargo fmt --all -- --check`; if it fails, format and repeat.
- [ ] Run `cargo xtask rust-file-length-lint --all` and focused Clippy checks.
- [ ] Run `cargo xtask smoke-test` and confirm it performs no live provider
      request.

## Milestone 5: Workspace Verification, Commit, And Review

Validate and publish the completed change. At the end of this milestone, the
branch is pushed, ordinary checks pass, live-provider results are known, and
review findings are ready for user decision without automatic fixes.

- [ ] Run `cargo clippy --workspace --all-targets --all-features`.
- [ ] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [ ] Run `cargo xtask check` and fix failures until it passes.
- [ ] Review `git diff origin/main...` for scope, stale docs, untracked files,
      secrets, generated media, and workflow safety.
- [ ] Move this plan from Active to Completed in `plans/README.md` only after
      every implementation and pre-push verification task is complete.
- [ ] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters and a descriptive body, and push the current branch without
      renaming it.
- [ ] On an eligible trusted run, require both Google and OpenAI live-image jobs
      to pass; if credentials or an eligible event are unavailable, report that
      as an explicit live-verification blocker rather than claiming coverage.
- [ ] Run `cargo xtask review` after the push against `origin/main`.
- [ ] Report every review finding without automatically fixing it; number each
      item, assign severity, explain image/CI context and the impact of doing
      nothing, provide lettered solution options, and recommend one option.
