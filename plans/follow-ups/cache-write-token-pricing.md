# Cache-write tokens get the plain input price

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Medium. **Confidence:** Medium. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-26, from branch `calummoore/adding-openai-decisions-api-9ler`, the options check of the plan for the shared judgment contract and OpenAI Decisions.
- **Location:** `crates/ai-interface/src/usage.rs:7`, `crates/ai-models-policy/src/pricing.rs:91`, `crates/ai-models-anthropic/src/anthropic/response.rs:49`
- **Reason:** The gap exists on `main` for Anthropic already, and the fix changes `ModelUsage`, which every adapter and caller builds or reads.

- **Terms:**
  - **`ModelUsage`:** the shared usage record in `crates/ai-interface/src/usage.rs`. It has input, output, cached input, reasoning, and total token counts.
  - **`price_usage`:** the pricing function in `crates/ai-models-policy/src/pricing.rs`. It multiplies each token count by a price that the host configures.
  - **Cache write:** the input tokens that a provider stores in its prompt cache during a request. Providers charge them at their own rate.
- **Background:**
  - `ModelUsage` has no field for cache-write tokens. `ModelUsageUnitKind` has no cache-write unit.
  - The Anthropic adapter adds `cache_creation_input_tokens` to `input_tokens`, at `crates/ai-models-anthropic/src/anthropic/response.rs:49-50`. They then get the plain input price.
  - OpenAI's Decisions API reports `usage.input_tokens_details.cache_write_tokens` inside `input_tokens`. The OpenAI guide says that Decisions charges cache writes at the standard `gpt-6-luna` cache-write rate, not at the plain input rate.
  - The planned OpenAI judgment adapter keeps cache-write tokens in the plain input count, as Anthropic does.
- **Problem:** `price_usage` prices cache-write tokens at the plain input price. Anthropic and OpenAI charge a different rate for them.
- **Impact if no action:** cost estimates are wrong for each request that writes to a prompt cache. For Anthropic, cache writes cost more than plain input, so the estimate is too low.
- **Root question:** how does a host get a correct cost estimate for tokens that a request writes to a prompt cache?
- **Options:**
  - **A.** Document the gap in `docs/protocol/model-policies.md` and keep the current counts. Effort: Small.
  - **B.** Add a `cache_write_input_tokens` count to `ModelUsage`, a cache-write unit kind, and a cache-write price to the pricing snapshot. Move Anthropic and the OpenAI judgment adapter to the new count. Effort: Medium.
- **Left out:** a price multiplier on the input price. The cache-write rate is not a fixed multiple of the input rate for every provider.
- **Wider fix:** B. Each provider that reports cache writes then maps them to one shared count.
- **Previous examples:** The Anthropic adapter, `crates/ai-models-anthropic/src/anthropic/response.rs:49-50`.
- **Recommendation:** B. Correct cost estimates need the count and the price, and the change is small enough to do before more adapters copy the gap.
- **Action:** ask the user, because option B changes the public `ModelUsage` type that every adapter and caller uses.
