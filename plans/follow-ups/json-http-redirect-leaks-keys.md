# Provider redirects can send API keys elsewhere

- **Severity:** High. **Effort:** Small. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/redesign-2-safety-net`. The options check of the MCP evaluation of `plans/ai-library-redesign.md` found it outside its scope.
- **Location:** `crates/json-http/src/reqwest_transport.rs:32-34`, `crates/ai-models-anthropic/src/anthropic/mod.rs:53`, `crates/ai-models-google/src/google/mod.rs:57`, `crates/ai-models-google/src/google/image_generation/client.rs:42`, `crates/ai-models-google/src/google/video_generation/client.rs:50`
- **Reason:** The problem is in the shared HTTP transport, outside the change that found it, and every fix adds a new failure for redirect responses, so the user decides.

- **Terms:**
  - **`json-http`:** the shared HTTP crate of this workspace, in `crates/json-http`. Every AI provider adapter sends its requests through it.
  - **Cross-host redirect:** an HTTP 3xx response whose `Location` names another host, port, or scheme.
- **Background:**
  - `ReqwestJsonHttpTransport::new` builds its reqwest client with the default redirect policy, so the client follows up to 10 redirects. A 307 or 308 redirect sends the same method and body again.
  - On a cross-host redirect, reqwest 0.13.4 removes only the `Authorization`, `Cookie`, `Cookie2`, `Proxy-Authorization`, and `WWW-Authenticate` headers, in `remove_sensitive_headers` of its `redirect.rs`.
  - The Anthropic adapter sends its key in `x-api-key`. The Google chat, image, and video adapters send their key in `x-goog-api-key`. reqwest keeps both headers on a cross-host redirect.
  - The MCP transport of `ai-mcp` already disables redirects, and `docs/protocol/ai-mcp.md` requires that it never sends authorization headers to a redirect target.
  - The redesign lets each provider config set its own base URL, so requests can also go to gateways that the host configures.
- **Problem:** When a provider endpoint or a configured gateway answers with a cross-host redirect, the transport sends the Anthropic or Google API key, and the request body, to the other host.
- **Impact if no action:** A redirect can send a paid API key to a host that the operator never configured.
- **Root question:** Where may a provider request, with its credentials, go after the first host?
- **Options:**
  - **A.** Disable automatic redirects in the provider transport. A 3xx response becomes a typed HTTP status error, as the MCP transport does. Risk: a provider or gateway that relies on a redirect stops working until its base URL is fixed. Effort: Small.
  - **B.** Follow same-origin redirects only, and stop at a cross-host redirect with a typed error. Same-origin redirects, such as a trailing-slash fix, keep working. Effort: Small.
  - **C.** Keep following redirects, and remove the adapter's own auth headers on a cross-host redirect. The transport must learn which headers carry credentials, through the auth trait of each adapter. Effort: Medium.
- **Left out:** Change nothing. It fails the goal, because the keys reach the other host.
- **Wider fix:** A. One transport default removes the class for every adapter and every new auth header. C must know each credential header, so a new header can leak again.
- **Previous examples:** The MCP transport of `ai-mcp` disables redirects for the same reason. `docs/protocol/ai-mcp.md` requires it.
- **Recommendation:** A. It removes the class at the shared transport, and the providers of this workspace do not use redirects in normal operation.
- **Action:** ask the user, because each option adds a new failure for redirect responses, and no recorded decision covers it.
