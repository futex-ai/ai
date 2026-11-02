# Live parity guard cannot fail

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/redesign-4-stream-parts`, item 76 (review finding 76).
- **Location:** `xtask/tests/live_models/event_guard_tests.rs:49`, `xtask/tests/live_models/provider_tests.rs:31`, `xtask/tests/live_models/event_tests.rs:90`, `docs/protocol/live-model-api-tests.md:134-136`, `xtask/README.md:67`, `docs/development.md:21`
- **Reason:** Every option removes the guard, and the recommended option also removes the protocol sentence that has required a credential-free parity check since before the AI library redesign, so the user decides.

- **Terms:**
  - **Live suite:** the paid tests under `xtask/tests/live_models`, which call the provider APIs with real keys.
  - **Parity:** the text events of a probe, joined, equal the response text.
  - **`CompletionEventExpectation`:** the enum that once selected the event checks of each probe: parity for synchronous calls and silence for deferred xAI calls.
- **Background:**
  - The redesign made deferred xAI calls emit model events, so the enum lost its `Silent` variant and has only `AssistantTextParity`. The validator ignores the enum argument.
  - The guard `every_provider_has_parity_in_synchronous_and_preferred_modes` compares the expectation of each provider with `AssistantTextParity`, so it always passes. A mutation probe skipped the synchronous xAI probe, skipped the event checks of deferred probes, or ran the catalog probes synchronously, and the guard still passed.
  - `docs/protocol/live-model-api-tests.md:134-136` says that credential-free tests enforce that every provider gets the synchronous parity probe. The sentence was on `main` before the redesign. `xtask/README.md:67` and `docs/development.md` repeat the claim.
  - A second guard case, `deferred_event_validation_accepts_deliberate_emission` (`event_guard_tests.rs:163`), repeats a case of `assistant_event_validation_accepts_reasoning_and_terminal_parity`.
- **Problem:** The guard and the docs report a check that does not exist.
- **Impact if no action:** A change of the live runner that skips a probe or its event checks passes unnoticed.
- **Root question:** How should the live suite keep the full event checks on every probe, and say truthfully what enforces them?
- **Options:**
  - **A.** Remove the guard, and keep the event checks. Effort: Small.
  - **B.** Remove the guard, the one-variant enum, its selector methods, and its unused arguments. Call the common validator directly. Effort: Small.
  - **C.** Apply B. Also remove the claim of a credential-free parity check from `live-model-api-tests.md`, the event-policy claims of `xtask/README.md` and `docs/development.md`, and the duplicate guard case. Keep the rule that every probe keeps text-event parity. Effort: Small.
  - **D.** Apply C, but keep the requested mode as an argument that the validator uses only in its failure texts. Effort: Small.
- **Left out:** A guard over a probe plan, a credential-free run of the runner, or a mutation-testing gate: each tests the internals of the test harness, which the `testing` skill excludes. A second enum variant: no requirement has a second mode.
- **Wider fix:** C. One unconditional validator call makes any new difference between the modes visible in code review.
- **Previous examples:** None found.
- **Recommendation:** C. B alone leaves three docs that describe a guard that no longer exists.
- **Action:** ask the user, because C removes a requirement of `live-model-api-tests.md` that no user decision covers.
