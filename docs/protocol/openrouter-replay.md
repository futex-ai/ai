# OpenRouter Reasoning And Conversation Replay Protocol

## Status And Purpose

Planned as part of the
[OpenRouter provider](openrouter-model-provider.md). Preserve provider-owned
assistant state across streaming, persistence, and tool continuation while
respecting the [offering continuity contract](model-offerings.md).

OpenRouter exposes plain reasoning and structured detail records. Those
records can contain summaries, signed text, and encrypted content and must
survive a tool continuation. See
[OpenRouter reasoning documentation](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens).
This adapter promises preservation of supported records, not conversion of
native provider-private state into an OpenRouter equivalent.

## Typed Shared State

`ai-interface` owns `ProviderConversationItem::OpenRouterAssistantMessage`
with the selected `OfferingKey`, provider model id, effective `ThinkingLevel`,
nullable assistant content, nullable plain reasoning, ordered detail records,
and ordered raw tool calls. Raw tool calls retain id, name, and the original
argument string. Model ids at this boundary are opaque adapter identifiers;
they do not replace the shared `CanonicalModel` registry.

`OpenRouterReasoningDetail` is a tagged enum whose variants and fields are:

- `Summary`: summary text, optional id, optional format, optional index;
- `Text`: text, optional signature, optional id, optional format, optional index;
- `Encrypted`: opaque data, optional id, optional format, optional index.

Use the wire tags `reasoning.summary`, `reasoning.text`, and
`reasoning.encrypted`. Preserve format as an opaque string identifier because
it belongs to the external API. Keep payloads typed; do not store a general
`serde_json::Value` replay object. Unknown record kinds are explicit typed
unsupported-replay failures, never ignored or exposed as assistant text.

## Stream Accumulation

The adapter normalizes `delta.reasoning` and the compatible
`delta.reasoning_content` alias. If both are present and equal, consume them
once; if both differ, fail with a typed conflicting-reasoning error. Preserve
absence separately from an empty string. Assistant content and tool fragments
follow the shared accumulator's indexing and validation rules.

Structured detail fragments are accumulated independently of plain reasoning:

1. An explicit index identifies a detail block within the primary choice. In
   its absence, an explicit id identifies the block. When a later fragment
   supplies both, reconcile them with the existing block; conflicting index/id
   associations are errors.
2. A record without either identifier is a new ordered block; do not guess that
   it continues another record merely because their kinds match.
3. Preserve first-seen block order. Append text, summary, signature, and opaque
   data fragments to their own fields. Retain kind, id, index, and format;
   incompatible repeated metadata fails. Null metadata does not erase a value
   already supplied. An incomplete block fails terminal validation.
4. Do not deduplicate, sort by content, trim, decode/re-encode encrypted data,
   regenerate signatures, or replace these records with reasoning text.
5. At terminal success, store the complete ordered sequence as replay state.
   Provider reasoning and all raw argument whitespace survive a serde round trip.

The shared `ChatCompletionsAccumulator` remains reusable for compatible
fields. Any extension to its DTOs or parsing must preserve existing provider
semantics and have their regression suites run. OpenRouter-specific state
merging belongs to the OpenRouter crate; raw JSON passthrough through core is
not an alternative to retaining typed fields.

## Public Events

Plain reasoning fragments produce `ReasoningTextDelta` exactly once. Structured
detail fragments remain terminal replay data in version one and do not emit
additional reasoning events; this avoids duplicate text when both forms are
returned. An encrypted/details-only stream therefore may have no public
reasoning events. Assistant fragments still emit incrementally, and their
concatenation must equal terminal assistant content.

When a chunk contains both plain reasoning and assistant content, emit
reasoning first. Schema-constrained requests emit no public events. Signatures,
ids, encrypted content, raw tool arguments, and provider formats are never
public completion text.

## Replay And Compatibility

Retain an OpenRouter assistant item whenever a successful response contains
reasoning state or dispatchable tools, including reasoning-bearing plain text
turns. A high-thinking Sonnet tool response requires a nonempty supported
reasoning-detail sequence; otherwise fail instead of returning a tool call
whose required continuation state was lost. An interruption does not return a
partial replay item or dispatchable tools.

On continuation, prefer the matching OpenRouter item over reconstructed
normalized assistant content/tool calls. Send its full detail sequence
unchanged. When details are present, omit the duplicate plain reasoning field
from the outgoing message; without details, replay the retained plain field.
Tool results follow their assistant message with the original matching ids,
including empty result strings. Never replay an internal operation id.

Validate the item's provider, catalog id, wire model id, and effective thinking
level against the current configured offering before authentication. A
mismatch returns `ModelError::IncompatibleConversation` with a typed reason.
The catalog fixes the upstream profile, so a different profile requires a
different offering key and cannot silently consume this context.

Native OpenAI/DeepSeek/Kimi/MiniMax/Qwen context must not be serialized as
OpenRouter state. OpenRouter rejects such foreign private replay context;
ordinary caller-authored assistant text and normalized tool history without
private replay can be used on an explicitly selected singleton route. Other
adapters ignore the OpenRouter item at their existing foreign-context
boundary; the new router prevents a conversation containing it from moving
to those adapters. No native-to-OpenRouter reasoning conversion is included.

`ModelRouteOrigin` is local routing metadata and is never sent to OpenRouter.
It persists even for responses without private replay data, ensuring that a
later turn is pinned after a successful initial fallback.

## Runtime, Logging, And Identity

The tool runtime retains complete replay data in its working conversation and
terminal checkpoint. Its logger-copy path removes the complete OpenRouter
assistant replay item from request and successful-response copies, on both
successful and failed calls. This must not mutate retained conversation state.
Route-origin metadata remains available for diagnostics.

Update `synthetic_tool_call_scope` to hash every new replay field, its variant,
and ordering, plus route origin. Different signatures, encrypted data, raw
arguments, model/profile selection, or reasoning must produce distinct test
scopes. Keep the public hashing interface stable and split the existing large
module into cohesive modules rather than exceeding the Rust file-size cap.

## Acceptance Tests

- Full and sparse records, nullable content, empty reasoning, optional fields,
  signed text, summaries, encrypted data, and ordered parallel tool calls.
- Fragmented reasoning/arguments/signatures/data and interleaved detail blocks,
  with index-only, id-only, reconciled identifiers, and unidentified records.
- Conflicting aliases/metadata, unknown types, incomplete blocks, missing
  required tool reasoning, malformed tool arguments, and terminal failures.
- Response-to-conversation-to-next-request round trips preserving private
  fields, tool ids, ordering, and argument whitespace.
- Plain reasoning without duplicated events, details-only reasoning, exact
  assistant parity, and structured-output event suppression.
- Matching versus mismatched catalog/model/thinking, foreign private context,
  local-origin omission from wire JSON, and no transport on incompatibility.
- Runtime retention and logger-copy redaction on success and failure, with
  deterministic identity tests for every new field.
- A two-turn live reasoning-tool probe through the public runtime, as specified
  by the provider protocol, without printing private replay values.
