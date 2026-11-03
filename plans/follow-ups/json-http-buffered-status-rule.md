# Buffered HTTP sends accept any status

- **Severity:** Medium. **Effort:** Medium. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-03, from branch `calummoore/redesign-5-policy-registry`: the blocked wider fix of review finding 90.
- **Location:** `crates/json-http/src/request.rs:119-131` (`send_value`, `send_bytes`, `send`), `crates/json-http/README.md`
- **Reason:** The fix adds stricter validation to the public `json-http` boundary, so the user decides.

- **Terms:**
  - **`json-http`:** the shared HTTP crate of this workspace, in `crates/json-http`. Every adapter sends its requests through it.
  - **Buffered send:** a request whose whole response body the caller reads at once, through `send_value`, `send_bytes`, or `send`.
- **Background:**
  - `docs/protocol/model-policies.md` says that an unexpected status below 400 gives the invalid output kind.
  - The streaming path of `json-http` already returns `Error::HttpStatus` for every non-success status. The buffered sends return every status as a successful response, and each caller checks the status itself.
  - On 2026-11-03, the fix of review finding 90 added one checked success boundary that each native adapter's buffered success path and binary download uses. Before that fix, a 302 response with an HTML body became a successful Google video result labelled `video/mp4`, and the Google and OpenAI image adapters and OpenAI transcription returned success for 3xx responses.
- **Problem:** Each buffered call site must remember to use the checked boundary. A new call site that reads `send_value()` and checks only `>= 400` brings the defect back.
- **Impact if no action:** A future adapter or modality can return redirect or informational content as generated output.
- **Root question:** Which layer makes sure that only a 2xx response reaches a success parser on the buffered HTTP path?
- **Options:**
  - **A.** Keep the checked boundary in the adapters only. Effort: Small.
  - **B.** Make the buffered sends of `json-http` return `Error::HttpStatus`, with the status, the headers, and a bounded body, for every non-2xx status, as the streaming path does. Put the check in the request builder, so that mocks, replay, and injected transports pass through it. The xAI deferred lifecycle then reads its 429 and 5xx retry statuses from the error. Update the `json-http` README. Effort: Medium.
- **Left out:** A content check on downloads: it lets a 3xx with a valid body through.
- **Wider fix:** B. It rejects every non-2xx status before any caller sees the response.
- **Previous examples:** Review finding 90 of 2026-11-03.
- **Recommendation:** B. One rule at the boundary that every buffered call passes.
- **Action:** ask the user, because B adds stricter validation to the public `json-http` boundary.
