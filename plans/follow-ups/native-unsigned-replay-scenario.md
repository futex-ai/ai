# No shared test for unsigned native replay

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/redesign-4-stream-parts`: the blocked wider fix of review finding 68.
- **Location:** `crates/ai-testkit/src/scenario.rs`, `crates/ai-testkit/recordings/`, `docs/protocol/conversation-parts.md` (section "Replay and skip rules")
- **Reason:** The fix adds a conformance scenario that tests a class of defects, and the `implementation-review` outcome rules reserve such tests for the user.

- **Terms:**
  - **Unsigned reply:** an Anthropic or Gemini reply without a thinking or thought signature.
  - **Conformance scenario:** a recorded request-and-response exchange that `ai-testkit` replays through every adapter, comparing each request with its recording.
- **Background:**
  - On 2026-11-02, `docs/protocol/conversation-parts.md` gained one replay rule for unsigned replies of native adapters: the joined text goes before the calls, as before the redesign. The Anthropic adapter keeps it with its normalized text replay. The Google adapter keeps it with a marker on replies whose layout differs.
  - The conformance scenarios of `ai-testkit` hold signed tool loops only, so no shared scenario covers an unsigned reply with text after a call. Each adapter has its own unit test.
- **Problem:** A later change to one native adapter can break the unsigned replay rule, and only that adapter's own unit test can catch it.
- **Impact if no action:** The two native adapters can drift apart on the same rule.
- **Root question:** How should the conformance suite hold the native adapters to one unsigned replay rule?
- **Options:**
  - **A.** Keep the per-adapter unit tests only. Effort: Small.
  - **B.** Add one conformance scenario (text, call, text, no signature) with recordings for Anthropic and Google. Each recording holds the literal continuation request that the rule gives. Effort: Medium.
- **Left out:** None.
- **Wider fix:** B.
- **Previous examples:** Review finding 68 of 2026-11-02, where unsigned Gemini continuations changed their request layout.
- **Recommendation:** B. One shared scenario checks the one rule for both adapters.
- **Action:** ask the user, because B adds a test for a class.
