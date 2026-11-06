# Test fixture streams succeed on 3xx statuses

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-03, from branch `calummoore/redesign-5-policy-registry`. A check of review finding 90 found it outside its scope.
- **Location:** `crates/ai-models-core/src/test_support.rs:138`
- **Reason:** The fixture behaved this way before the AI library redesign, so the fix waits for a branch that works on the test support.

- **Terms:**
  - **`buffered_fixture`:** a test helper in `ai-models-core` that turns a buffered response fixture into an SSE stream for adapter tests.
- **Background:** The real SSE transport of `json-http` returns `Error::HttpStatus` for every non-success status. `buffered_fixture` turns a buffered fixture with a 3xx status into a successful SSE stream.
- **Problem:** A test that uses a 3xx fixture sees a successful stream, which production never produces.
- **Impact if no action:** A test can pass on a status that the real transport rejects, and so miss a status-handling defect.
- **Root question:** How should the test fixture treat a non-success status?
- **Options:**
  - **A.** Make `buffered_fixture` return the same `Error::HttpStatus` as the real transport for every non-2xx status. Update the tests that relied on the old behaviour. Effort: Small.
  - **B.** Remove the status from buffered fixtures, and require a separate error fixture for failures. Effort: Medium.
- **Left out:** None.
- **Wider fix:** A. The fixture then follows the production transport rule.
- **Previous examples:** None found.
- **Recommendation:** A.
- **Action:** fix after merge, apply A, because the fixture behaved this way before the redesign.
