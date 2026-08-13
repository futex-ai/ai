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

## Milestone 2: Protocol Review Follow-Up

Resolve the initial plan review before implementation. At the end of this
milestone, local verification, retry timing, provider quality behavior, and the
maximum billable-attempt budget are unambiguous.

- [x] Provide a complete credential-free test command that cannot retain the
      ignored live-test filter accidentally.
- [x] Specify that `Low` quality is requested where supported and budget for up
      to three billable attempts after transient failures.
- [x] Make `STANDARD_TRANSIENT_RETRY_DELAYS` normative and define the adapter
      and workflow timeout bounds.

## Milestone 3: Central Live-Image Test Harness

Build the ignored integration suite and its credential-free seams. At the end
of this milestone, both production image adapters can be exercised through one
catalog-driven runner without making live calls during ordinary tests.

- [x] Add failing registry tests proving every catalog entry advertising
      `ImageGeneration` belongs to a registered live-image provider, every
      registered provider has at least one image entry, and non-image entries
      are excluded.
- [x] Implement a focused `LiveImageProvider` registry for provider identity,
      catalog access, workflow test/secret names, authentication, and
      construction of `DynImageGenerator` from the catalog upstream model id.
- [x] Add failing runner tests for strict non-empty
      `LIVE_IMAGE_API_KEY` handling, sequential catalog iteration, aggregated
      failures, and provider-neutral dynamic dispatch.
- [x] Add failing retry tests proving only `RateLimited` and
      `TransientProvider` receive bounded retries and all terminal error classes
      stop immediately.
- [x] Add failing validation tests for provider/model identity, non-empty
      payloads, supported MIME types, and PNG/JPEG/WebP signature agreement.
- [x] Implement the safe square, low-quality, one-image probe and ensure logs
      contain identifiers and concise failures but never image bytes or
      credentials.
- [x] Add ignored `catalog_tests::google_image_catalog` and
      `catalog_tests::openai_image_catalog` tests that call the production
      adapters through the shared runner.
- [x] Remove the superseded provider-local ignored live smoke tests while
      retaining their deterministic transport and response tests.
- [x] Keep source and test modules cohesive and every changed Rust file below
      the 300-line cap.

## Milestone 4: Credentialed CI Workflow

Wire the suite into GitHub Actions with explicit security and spend limits. At
the end of this milestone, eligible CI events request one image per attempt for
each image catalog model and fail visibly on missing coverage or credentials.

- [x] Add failing workflow guard tests for every registered provider test and
      secret, trusted pull-request gating, default-branch schedule/manual
      gating, read-only permissions, and the exact ignored-test command.
- [x] Create `.github/workflows/live-images.yml` with independent Google and
      OpenAI matrix entries, `fail-fast: false`, `max-parallel: 1`, bounded job
      timeout, and superseded-run cancellation.
- [x] Expose only the current provider secret as `LIVE_IMAGE_API_KEY`, require
      it before checkout/test execution, and fail rather than silently skip
      eligible coverage.
- [x] Run only for same-repository pull requests not authored by Dependabot,
      daily on the default branch, and manual dispatch without using
      `pull_request_target`.
- [x] Confirm the workflow never uploads generated image artifacts and that
      ordinary CI, `cargo xtask check`, and `cargo xtask smoke-test` remain
      credential-free.

## Milestone 5: Documentation And Focused Verification

Document the implemented boundary and prove the harness without spending API
credits during normal development. At the end of this milestone, maintainers
can discover, run, and safely extend live image coverage.

- [x] Mark the live-image protocol implemented and align the image-generation
      and live-model protocols with the new suite.
- [x] Update the workspace and `xtask` READMEs with CI cadence, provider/model
      discovery, billable local commands, exclusions, and key code locations.
- [x] Add extension guidance requiring each future image provider to register
      its catalog, adapter construction, CI test, and secret mapping.
- [x] Run the credential-free `live_images` integration tests and focused
      Google/OpenAI image adapter tests with a 100% pass rate.
- [x] Run `cargo fmt --all -- --check`; if it fails, format and repeat.
- [x] Run `cargo xtask rust-file-length-lint --all` and focused Clippy checks.
- [x] Run `cargo xtask smoke-test` and confirm it performs no live provider
      request.

## Milestone 6: Workspace Verification, Commit, And Review

Validate and publish the completed change. At the end of this milestone, the
branch is pushed, ordinary checks pass, live-provider results are known, and
review findings are ready for user decision without automatic fixes.

- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for scope, stale docs, untracked files,
      secrets, generated media, and workflow safety.
- [x] Move this plan from Active to Completed in `plans/README.md` only after
      every implementation and pre-push verification task is complete.
- [x] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters and a descriptive body, and push the current branch without
      renaming it.
- [x] On an eligible trusted run, require both Google and OpenAI live-image jobs
      to pass; if credentials or an eligible event are unavailable, report that
      as an explicit live-verification blocker rather than claiming coverage.
- [x] Run `cargo xtask review` after the push against `origin/main`.
- [x] Report every review finding without automatically fixing it; number each
      item, assign severity, explain image/CI context and the impact of doing
      nothing, provide lettered solution options, and recommend one option.

## Live Verification Status

On 2026-08-13, the OpenAI `gpt-image-2` catalog probe passed locally through
the production adapter. Google live verification remains blocked: this
workspace has no `GOOGLE_API_KEY`, and the branch has no open pull request that
could trigger an eligible trusted workflow run. This status does not claim a
Google live pass.

## Post-Push Review Status

The initial 2026-08-13 post-push review reported two P3 findings: the exclusion
test assumed every provider had a non-image catalog entry, and the split
integration-test target did not use the required directory-root module layout.
No review finding was automatically fixed. The user subsequently selected
option A for both findings, authorizing the corrections in Milestone 7.
The follow-up post-push review of commit `7e3d001` found no functional defects.

## Milestone 7: User-Selected Review Follow-Up

Implement the user's selected option A for both P3 review findings. At the end
of this milestone, image-only provider catalogs are valid, the split integration
target follows the required module layout, and the corrected branch has passed
the complete publish-and-review workflow.

- [x] Add a failing regression test proving an image-only catalog remains valid.
- [x] Replace the per-provider non-image requirement with a global mixed-catalog
      exclusion check.
- [x] Add a failing layout guard for the prohibited split-root test structure.
- [x] Register `tests/live_images/mod.rs` as the integration target, use normal
      module declarations, and keep all test bodies in `*_tests.rs` leaves.
- [x] Align workflow test names, local commands, README references, and protocol
      extension guidance with the directory-root target.
- [x] Run focused live-image tests, strict Clippy, formatting, file-length lint,
      smoke tests, workflow lint, and diff validation.
- [x] Run the full workspace Clippy and test suites with a 100% pass rate.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review the complete diff, move this plan back to Completed, run
      `git add -A`, commit with a Conventional Commit message, and push.
- [x] Run `cargo xtask review` after the push and report every finding without
      automatically fixing it.
