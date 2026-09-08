# OpenRouter Model Provider And Offering Selection

## Summary

Add OpenRouter as a provider and let callers select one canonical model across
native and OpenRouter offerings with explicit thinking intent and deployment
priority. Preserve each offering's capabilities, provider replay, and actual
account charge. Automatic fallback is limited to an initial retryable failure;
retained conversations stay on the offering that successfully served them.

This is a plan for future implementation. Creating this file does not deliver
the provider or change runtime behavior. The complete target contracts are:

- [Model offerings and routing](../docs/protocol/model-offerings.md)
- [OpenRouter provider](../docs/protocol/openrouter-model-provider.md)
- [OpenRouter replay](../docs/protocol/openrouter-replay.md)
- [Provider-reported model cost](../docs/protocol/model-usage-cost.md)

## Scope And Decisions

- Keep `KnownModelSpec` per offering and add typed `CanonicalModel` identity.
  Existing catalog ids remain stable; canonical identity is a curated release
  mapping rather than a claim of identical weights.
- Move thinking types to `ai-interface`; provide exact selection and explicit
  at-most resolution that freezes one effective level before fallback.
- Implement `ConfiguredModelRouter` in new `ai-models-router`, over injected
  `DynModel` registrations. Deployment priority expresses provider preference.
- Add `ai-models-openrouter` with disabled/high Sonnet 4.6 and disabled Llama
  3.3 70B. Fixed Anthropic/Groq upstream profiles make the capability boundary
  explicit; internal OpenRouter fallback is disabled initially.
- Retain typed reasoning records and route origins through the tool runtime.
  Separate reported account cost from configured token-price estimates.
- Include deterministic regression coverage, smoke tests, live catalog/tool
  probes, workspace docs, a runnable composition example, and CI wiring.

Dynamic catalogs, arbitrary models or upstream profiles, native reasoning
translation, switching an active conversation between offerings, application
UI/configuration changes, and Firna deployment are excluded. Firna receives a
documented adoption boundary; it is not a hidden prerequisite for a working
router in this workspace. No UI/mockup milestone is needed.

## Milestone 1: Typed Identity And Selection Contracts

At the end of this milestone all existing providers still work, every current
catalog entry has canonical identity, and callers can express precise typed
selection without changing existing catalog ids or constructor behavior.

- [ ] Add failing tests for canonical parsing/display/serde, explicit stable
      acronym spellings, unknown-value rejection, and the complete registry.
- [ ] Add `ai_interface::model_identity` with `CanonicalModel`, `CatalogModelId`,
      `OfferingKey`, `ThinkingLevel`, and `ThinkingRequirement` at their real
      owner path, with module/API docs and external `_tests_` modules.
- [ ] Move `ThinkingLevel` out of core, preserve existing ordering/values, add
      serde, and update every import instead of adding compatibility re-exports.
- [ ] Add canonical/thinking route requirements and typed invalid-requirement
      reasons; cover exact, at-most, missing, identical, and conflicting inputs.
- [ ] Add `KnownModelSpec.canonical_model` and its offering-key helper; migrate
      all eight provider catalogs, mock/specialized entries, fixtures, examples,
      and construction tests without changing existing catalog metadata.
- [ ] Add fallible catalog registration with typed duplicate-key errors;
      migrate every current caller and remove the permissive registration path.
- [ ] Test that multiple native thinking variants share a canonical identity
      while retaining unique offering keys and independent features/limits.
- [ ] Add typed route-origin context and conversation/response identity errors;
      test serde, legacy payloads, foreign-context handling, and scope hashing.
- [ ] Split large routing/hash modules as needed; keep changed Rust files under
      300 lines and module roots free of runtime implementation.
- [ ] Update interface/core and affected provider READMEs. Clarify the future
      route's exact thinking contract alongside existing constructor downgrades.
- [ ] Run formatting, focused interface/core/catalog tests, workspace Clippy,
      and all-feature workspace tests; require all executed tests to pass.

## Milestone 2: Authoritative Usage Accounting

At the end of this milestone existing providers retain their pricing behavior,
and any adapter can report an account charge that survives configured pricing
wrappers and is distinguishable from an unknown or estimated amount.

- [ ] Add failing serde/effective-cost tests for `ProviderReportedCost`,
      `ModelCostSummary`, `ModelCostSource`, absent reports, and reported zero.
- [ ] Add optional `ModelUsage.provider_cost` and the pure effective-cost
      accessor with the exact precedence and bucket-completeness rules.
- [ ] Migrate current usage literals/mocks and preserve legacy deserialization.
- [ ] Add failing pricing-wrapper regressions for authoritative nonzero/zero
      reports, separate upstream cost, partial/free estimates, duplicate or
      missing bucket lines, and repeated wrappers.
- [ ] Preserve the provider report in `price_usage`, `complete`, and
      `complete_with_events`; never distribute a reported total into invented
      token prices or replace it with configured estimates.
- [ ] Cover runtime terminal-checkpoint/logger retention and event parity.
- [ ] Update interface/core READMEs with the accessor and consumer migration
      boundary; keep external billing/storage adoption explicitly downstream.
- [ ] Run formatting, focused interface/core/tool-runtime tests, workspace
      Clippy, and all-feature workspace tests.

## Milestone 3: Complete OpenRouter Adapter

At the end of this milestone callers can construct all curated OpenRouter
offerings and complete streamed text, supported image, structured, reasoning,
and tool-continuation calls through `DynModel`, with normalized usage/errors.

- [ ] Recheck the cited OpenRouter model endpoints and control/reasoning/cost
      references. Reconcile any API drift in the protocols before coding it;
      do not substitute a different model or weaken a capability silently.
- [ ] Scaffold `ai-models-openrouter` with thin roots, separate catalog,
      configuration, request, response, stream, replay, and test modules.
- [ ] Add workspace membership/internal dependencies manually with workspace
      references; add necessary external dependencies using `cargo add` without
      a version, and build the new package immediately.
- [ ] Add failing `openrouter` provider parsing/display/serde tests, then add
      `ProviderKind::OpenRouter` and update every exhaustive provider match.
- [ ] Add exact catalog/profile tests for all three entries, canonical/wire
      mappings, thinking levels, limits, features, and internal ranking tiers.
- [ ] Implement validated constructors using catalog ids and injected HTTP/auth
      only; reject unknown selections before auth/transport.
- [ ] Add failing final-request tests for the fixed endpoint, authentication,
      stream mode, restricted upstream, disabled internal fallback, and required
      parameter support. Reject arbitrary wire/model/profile overrides.
- [ ] Implement text/role/name/tool-result and Sonnet image mapping; test empty
      content, ordered parts, unsupported images/video, and blank system text.
- [ ] Add control tests before mapping sampling, output limits, stops, thinking,
      deferred rejection, strict/automatic tool choices, and invalid tool names.
- [ ] Add structured-output tests before implementing JSON-object mode, schema
      instructions, local validation, finish handling, and event suppression.
- [ ] Add typed OpenRouter assistant/detail/raw-tool context in `ai-interface`
      and tests for every variant, optional field, unknown kind, and round trip.
- [ ] Implement OpenRouter normalization around reusable core accumulation;
      retain reasoning aliases/details, costs, generation ids, and model ids.
- [ ] Add failing stream tests for text/reasoning/tool fragmentation, final
      usage-only chunks, missing markers, partial EOF, malformed payloads,
      returned-model mismatch, and pre-/post-progress errors.
- [ ] Implement detail-block ordering/merging and alias-conflict rules exactly
      as specified; test signatures/encrypted data without logging payloads.
- [ ] Test and implement dispatchable-tool validation, unique ids, raw argument
      preservation, terminal tool suppression, and required reasoning state.
- [ ] Prove complete response-to-next-request tool replay; reject foreign or
      mismatched model/catalog/thinking context locally and omit route origins
      from wire JSON.
- [ ] Extend scope hashing and existing-provider foreign-context regressions.
      Add success/error logger-redaction tests before changing runtime copies.
- [ ] Add exact decimal-cost conversion and non-overlapping token regressions;
      implement reported account/upstream cost using the shared accounting DTO.
- [ ] Test status and gateway-code errors, no substring classification,
      interrupted-stream policy, credential redaction, and internal sources.
- [ ] Prove buffered/event terminal parity and separate reasoning events;
      details-only reasoning remains replay state without duplicate events.
- [ ] Atomically add OpenRouter to smoke construction, the live provider
      registry, workflow matrix, and credential-free coverage guards so adding
      the provider enum does not leave exhaustive integration checks broken.
- [ ] Write the publishable crate README with required sections and compiling
      examples; update affected shared/provider docs as behavior lands.
- [ ] Run the new crate build/tests/Clippy, relevant shared/other-provider replay
      regressions, formatting, all-feature workspace tests, and smoke tests.

## Milestone 4: Working Offering Router And Safe Fallback

At the end of this milestone callers select canonical models through a real
`DynModelRouter`, initial retryable failures can use the next offering, and
tool conversations stay on their successful offering without shared pin state.

- [ ] Scaffold/build `ai-models-router` with core/interface/multi dependencies,
      a publishable README, and `ConfiguredModelRouter` behind `ModelRouter`.
- [ ] Test registration validation with `unimock` models: unknown/duplicate
      keys, empty configuration, and specialized generation-model rejection.
- [ ] Implement registration of already constructed models and explicit
      priority. No provider imports, credentials, environment reads, or network
      lookup belong in this crate.
- [ ] Test every existing hard filter/preference plus canonical/exact/at-most
      selection, highest-eligible-level freezing, contradictions, and no match.
- [ ] Implement deterministic lexicographic ranking and stable key ties; test
      reversed provider priorities and arbitrary insertion order.
- [ ] Add failing `MultiModel` tests for `AnyError` and `RetryableOnly`, then
      add the explicit policy constructor while preserving existing defaults.
- [ ] Test each shared model-error class and prove the router stops after an
      interruption or terminal failure, including a provider that incorrectly
      returns a retryable error after public text events.
- [ ] Validate successful lane identity and attach exactly one route origin;
      test that a mismatched adapter response cannot be treated as success.
- [ ] Implement request-derived pinning to the exact retained offering. Test
      unavailable/ineligible/conflicting origins and untagged private/tool
      histories before transport, plus the explicit provider-and-id singleton
      exception and unconditional private-context provider ownership checks.
- [ ] Drive a real tool-runtime conversation with mocked provider transports:
      native initial rate limit, OpenRouter success/tool call, exact OpenRouter
      continuation, and pinned failure without native retry.
- [ ] Test shared-router concurrent conversations, wrapper order, provider-cost
      preservation, structured silence, and public terminal/event parity.
- [ ] Add the runnable `openrouter_fallback` example with native/OpenRouter
      Sonnet registration, explicit priorities, canonical/thinking requests,
      and an OpenRouter-only Llama selection. Read credentials only there.
- [ ] Update core/multi/interface/router docs and planned notes in the controls
      and completion protocols to reflect the implemented boundaries.
- [ ] Run new/existing router and fallback tests, runtime integration tests,
      example builds, formatting, workspace Clippy/tests, and smoke tests.

## Milestone 5: Live Acceptance And Consumer Documentation

At the end of this milestone live probes cover every advertised offering and
the reasoning-tool lifecycle, and a consumer can configure the library from
its public documentation without relying on an unspecified downstream router.

- [ ] Complete OpenRouter catalog/event/identity/thinking/token/cost assertions
      in the ignored live suite, including legitimate reported zero cost.
- [ ] Add the high-Sonnet two-turn reasoning/tool probe and guards for its
      replay/cost exception to the ordinary connectivity suite's scope.
- [ ] Verify the `OPENROUTER_API_KEY` matrix entry, missing-secret failure,
      trusted-PR conditions, and isolation from credential-free check commands.
- [ ] Run `cargo test --locked -p xtask --test live_models` credential-free.
- [ ] With explicit credentials, run `openrouter_catalog` against every entry
      and the reasoning-tool probe. Record failures individually; unavailable
      credentials/endpoints leave live acceptance pending, not passed.
- [ ] Run the composition example with real configured credentials for both
      overlapping Claude selection and OpenRouter-only coverage. Deterministic
      transport tests, not deliberately broken production keys, prove fallback.
- [ ] Update root/crate READMEs and live/control/stream/event protocols with
      exact delivered behavior, public commands, ownership, and limitations.
- [ ] Document Firna's later revision/configuration, route-origin retention,
      effective-cost accessor, and thinking-import migration without claiming
      those downstream changes were made here.
- [ ] Audit all new API items, typed error boundaries, injected trait objects,
      tests outside production files, file sizes, source imports, and docs.

## Milestone 6: Verification, Commit, Push, And Review

At the end of this milestone the full implementation is checked, committed,
pushed, and reviewed against `origin/main`, with findings presented for the
user's decision. Do not automatically fix review findings.

- [ ] Run `cargo fmt --all -- --check`; format and repeat if necessary.
- [ ] Run `cargo xtask rust-file-length-lint --all`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features`.
- [ ] Run `cargo test --workspace --all-features` and require all tests to pass;
      retain separate evidence of the credentialed acceptance results.
- [ ] Run `cargo xtask smoke-test` and the new runnable example as applicable.
- [ ] Run `cargo xtask check`; resolve failures and rerun affected checks.
- [ ] Validate changed Markdown/local links and audit the complete diff against
      `origin/main`, including new source, tests, docs, dependencies, and CI.
- [ ] Run `git add -A`, then commit all completed work with a Conventional
      Commit title of at most 50 characters and a descriptive body.
- [ ] Push the current branch without renaming it.
- [ ] Run `cargo xtask review` after the push against `origin/main`.
- [ ] Report every finding with a number, severity, codebase/feature context,
      impact of doing nothing, lettered solution options, and a recommendation.
      Leave review-driven changes for the user to authorize.
- [ ] Only after all milestones and live acceptance are complete, mark protocol
      status implemented and move the plan from active to completed. Publish
      the final documentation status using Markdown validation, commit, push,
      and review; report any further findings without automatic fixes.

## Documentation-Only Publication

Initial plan/spec publication requires Markdown/link validation and diff review,
followed by `git add -A`, a Conventional Commit, push, and `cargo xtask review`.
It does not require `cargo xtask check` under the repository's documentation-only
exception. Keep all implementation milestones unchecked and the plan active.
