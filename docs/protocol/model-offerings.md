# Model Offerings And Routing Protocol

## Status And Purpose

Planned; no implementation is claimed by this document. The
[OpenRouter implementation plan](../../plans/openrouter-model-provider.md)
delivers this contract. Existing provider APIs retain their documented behavior
until the corresponding milestones land.

Select a curated model through one or more explicitly configured providers
while preserving thinking intent, capability requirements, and conversation
continuity. A canonical model is a curated release designation; it does not
certify identical weights, token budgets, quality, or replay formats.

## Ownership

- `ai-interface` owns canonical identity, thinking requirements, offering keys,
  route-origin conversation metadata, and shared routing/model errors.
- `ai-models-core` owns offering metadata and catalog validation/lookups.
- Provider crates own wire identifiers, capabilities, and validated adapters.
- New `ai-models-router` implements `ModelRouter` over registered `DynModel`
  offerings. It depends on interface/core/multi, never concrete providers.
- `ai-models-multi` owns ordered execution and selectable failure policy.
- Composition roots construct adapters, inject credentials and wrappers, assign
  priorities, and pass the resulting router as `DynModelRouter`.

There is no environment lookup, credential discovery, dynamic catalog fetch,
provider construction, or health probing inside catalog/router code.

## Identity

Use the new public owner module `ai_interface::model_identity` for canonical
identity, offering keys, and thinking types; update consumers to that path.
Add `CanonicalModel` as a shared enum with explicit stable snake-case serde
names, `as_str`, display, and exact configuration parsing. Unknown values fail
parsing. Provider constants refer to this single registry; they never infer
identity by trimming strings or importing another provider's adapter crate.

`KnownModelSpec` remains one deployable offering variant. Add
`canonical_model: CanonicalModel`; retain provider, catalog id, wire id,
thinking level, capabilities, context limit, intelligence, speed, and cost on
each offering. Existing catalog strings and metadata do not change in this
migration. In particular, intelligence and reasoning support are not moved to
the canonical enum.

An `OfferingKey` contains `provider: ProviderKind` and
`catalog_id: CatalogModelId`. `CatalogModelId` is an opaque, owned string type
with a private field, nonblank construction, exact case-sensitive equality,
and string serde. It identifies a configuration/catalog boundary, not a
canonical model. Existing static catalog constants remain usable through a
`KnownModelSpec::offering_key()` helper. Wire identifiers remain adapter-owned
external identifiers and are never used to infer canonical equivalence.

Uniqueness is enforced on the offering key. Multiple thinking levels and
multiple offerings from the same provider may share a canonical model.
Duplicate keys are rejected even when their metadata is identical. Add a
fallible catalog-registration path and migrate current registration callers;
do not keep a second path that silently permits duplicate keys.

## Initial Canonical Registry

All current `known_models()` entries are covered, including mock and specialized
generation entries. Thinking variants inherit the row for their actual wire id.
This table maps repository identifiers; it is not an availability claim about
external APIs. Config values are written explicitly rather than inferred from
Rust acronym casing.

| Canonical enum | Config value | Existing provider wire id |
| --- | --- | --- |
| `Mock` | `mock` | `mock` |
| `ClaudeSonnet5` | `claude_sonnet_5` | `claude-sonnet-5` |
| `ClaudeOpus5` | `claude_opus_5` | `claude-opus-5` |
| `ClaudeFable5` | `claude_fable_5` | `claude-fable-5` |
| `ClaudeSonnet46` | `claude_sonnet_4_6` | `claude-sonnet-4-6` |
| `ClaudeOpus47` | `claude_opus_4_7` | `claude-opus-4-7` |
| `ClaudeHaiku45` | `claude_haiku_4_5` | `claude-haiku-4-5` |
| `DeepSeekV4Pro` | `deepseek_v4_pro` | `deepseek-v4-pro` |
| `DeepSeekV4Flash` | `deepseek_v4_flash` | `deepseek-v4-flash` |
| `Gemini36Flash` | `gemini_3_6_flash` | `gemini-3.6-flash` |
| `Gemini35FlashLite` | `gemini_3_5_flash_lite` | `gemini-3.5-flash-lite` |
| `Gemini31FlashImage` | `gemini_3_1_flash_image` | `gemini-3.1-flash-image` |
| `Veo31GeneratePreview` | `veo_3_1_generate_preview` | `veo-3.1-generate-preview` |
| `KimiK3` | `kimi_k3` | `kimi-k3` |
| `MiniMaxM3` | `minimax_m3` | `MiniMax-M3` |
| `MiniMaxM27` | `minimax_m2_7` | `MiniMax-M2.7` |
| `MiniMaxM27Highspeed` | `minimax_m2_7_highspeed` | `MiniMax-M2.7-highspeed` |
| `Gpt56Sol` | `gpt_5_6_sol` | `gpt-5.6-sol` |
| `Gpt56Terra` | `gpt_5_6_terra` | `gpt-5.6-terra` |
| `Gpt56Luna` | `gpt_5_6_luna` | `gpt-5.6-luna` |
| `Gpt55` | `gpt_5_5` | `gpt-5.5` |
| `Gpt54Mini` | `gpt_5_4_mini` | `gpt-5.4-mini` |
| `Gpt54Nano` | `gpt_5_4_nano` | `gpt-5.4-nano` |
| `GptImage2` | `gpt_image_2` | `gpt-image-2` |
| `Sora2` | `sora_2` | `sora-2` |
| `Qwen37Max` | `qwen_3_7_max` | `qwen3.7-max` |
| `Qwen37Plus` | `qwen_3_7_plus` | `qwen3.7-plus` |
| `Qwen37Flash` | `qwen_3_7_flash` | `qwen3.7-flash` |
| `Grok45` | `grok_4_5` | `grok-4.5` |
| `Grok420` | `grok_4_20` | `grok-4.20` |
| `Grok420Reasoning` | `grok_4_20_reasoning` | `grok-4.20-reasoning` |
| `Llama3370bInstruct` | `llama_3_3_70b_instruct` | OpenRouter-only addition |

Distinct non-thinking wire ids remain separate unless a documented mapping
establishes equivalence. In particular, this migration does not equate MiniMax
highspeed or Grok reasoning ids with neighboring entries. Moving aliases such
as `latest`, automatic routers, and cross-model fallbacks are excluded from
new canonical mappings. An upstream release replacement requires a new
canonical value; changing a wire spelling alone preserves the documented key.

## Thinking And Route Resolution

Move `ThinkingLevel` from core to its real owner in `ai-interface`, preserving
its ordering and stable values. Update imports and remove the old core export.
Add `ModelRequirement::CanonicalModel(CanonicalModel)` and
`ModelRequirement::Thinking(ThinkingRequirement)`, where `ThinkingRequirement`
is `Exact(ThinkingLevel)` or `AtMost(ThinkingLevel)` with snake-case serde.

A canonical-model requirement must include a thinking requirement. Missing or
conflicting canonical/thinking requirements produce a typed
`ModelRouterError::InvalidRequirements` with an enum reason. Identical repeated
requirements are harmless. Existing model-id/provider/feature/context
requirements remain conjunctive filters.

`ConfiguredModelRouter` accepts a catalog and registrations containing an
offering key, already constructed `DynModel`, and `priority: u32`. Construction
rejects unknown keys, duplicate registrations, and image/video generation
entries; these use specialized traits. Registrations are the only eligible
models. Empty registration returns `NoModelsConfigured` on resolution.

Resolution performs these steps without transport:

1. Apply all ordinary hard requirements to registered catalog metadata.
2. For `Exact`, keep precisely that effective thinking level. For `AtMost`,
   discard higher levels, then keep only the highest remaining level across
   eligible offerings. The resolved chain has one effective level; it never
   walks through progressively weaker thinking settings after failures.
3. Return `NoModelsMatched` when no offering survives.
4. Apply preferences lexicographically in supplied order: priority ascending,
   intelligence descending, speed descending, cost ascending, context
   descending, and preferred-feature presence first. An empty preference list
   uses deployment priority. Break remaining ties by provider config string
   then catalog id, both ascending; insertion order does not affect routing.
5. Return an immutable request-aware `DynModel` route, including singletons.

There is no `DirectProvider` preference. Priority 10 for Anthropic and 20 for
OpenRouter expresses native-first selection; swapping priorities reverses it.
Existing direct constructors keep their documented downgrade behavior. A
registration must represent its effective catalog level, and a successful
response must match its registered provider, catalog id, wire id, and thinking
level. A mismatch is a typed model contract failure and is not retried.

Example serialized request, with both disabled Sonnet offerings registered:

```json
{
  "requirements": [
    { "canonical_model": "claude_sonnet_4_6" },
    { "thinking": { "exact": "disabled" } }
  ],
  "preferences": ["deployment_priority"]
}
```

## Conversation Continuity And Fallback

Add `ProviderConversationItem::ModelRouteOrigin` containing the offering key,
canonical model, and effective thinking level. Each successful routed response
receives exactly one such item after identity validation. The tool runtime
already retains response context in assistant messages; preserve that behavior
for plain text, reasoning, and tool responses. Adapters never serialize this
workspace-owned item upstream. It is safe diagnostic metadata and is retained
in logger copies and included in deterministic tool-call scope hashing.

On every call, the route examines retained assistant context. All origin items
must agree, and all private replay items must belong to the selected provider.
If an origin exists, call only that exact offering, provided it is
still registered and satisfies the resolved selection. Never substitute
another provider, model, or thinking level when the pinned offering fails.
This pin is derived from the request, not mutable state on the shared router;
independent conversations may safely use the same `DynModel` concurrently.

Conflicting origins, an unavailable/ineligible origin, or a request containing
tool history or provider replay without an origin returns
`ModelError::IncompatibleConversation` with a typed reason before auth or
transport. The legacy exception requires both `Provider` and `ModelId`
requirements selecting exactly one offering; an accidentally singleton
canonical route does not qualify. It accepts untagged history only when all
provider-owned items belong to its provider. OpenRouter performs
its additional model/profile replay checks. Caller-authored text-only history
without replay or tool state may start a new routed conversation.

For an unpinned request, try ranked offerings in order only after
`RateLimited` or `TransientProvider`. Stop on `Interrupted`, internal,
unsupported-control, context-limit, and other terminal errors. Return the last
attempted error if eligible fallbacks are exhausted. No emitted completion
events may be followed by an automatic new attempt under this policy.

Add a pure `FallbackPolicy` enum to `ai-models-multi`: `AnyError` preserves
`MultiModel::new` behavior; `RetryableOnly` implements the rule above through an
explicit constructor. On the observing path, public text followed by even an
incorrectly classified retryable error must stop as `Interrupted`, never start
another lane. The router uses `RetryableOnly`, passes requests and events
unchanged, and attaches origin per successful lane. Existing restart
semantics remain for the legacy `AnyError` policy. Retry/concurrency/pricing
wrappers are configured per offering before registration. Deadlines remain
per adapter invocation, as in the existing controls contract.

Switching a retained conversation to another offering requires an explicit
caller-owned restart policy outside this initial contract. The implementation
must not discard reasoning or reconstruct tool history to force a switch.

## Integration And Verification

`ai-models-router/examples/openrouter_fallback.rs` is the runnable composition
root in this workspace. It reads explicit environment credentials, registers
native/OpenRouter Sonnet offerings and OpenRouter Llama, and resolves through
`DynModelRouter`; it is never part of credential-free tests. Firna adoption is
a separate downstream revision/configuration change: register its existing
wrapped models, map canonical/thinking requests, and preserve route origins.
Shipping this workspace does not claim Firna deployment or config changes.

Required tests cover registry mappings/serde, duplicate keys, thinking filters,
stable ranking, unavailable offerings, identity mismatches, per-error fallback,
origin retention, legacy singleton history, conflicting origins, no cross-
provider replay, event parity, and concurrent independent conversations.
An integration test must drive initial fallback and a subsequent tool result
through the real `ToolCallingRuntime` using mocked provider transports.
