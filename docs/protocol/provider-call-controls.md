# Provider Call Controls Protocol

## Purpose

Define portable generation and execution intent for every
`ai_interface::Model` call while leaving provider wire formats and completion
lifecycles inside their owning `ai-models-*` adapters.

## Ownership

- Callers choose model, routing, retry, concurrency, pricing, generation
  intent, effort, completion preference, and total deadline.
- `ai-interface` owns the typed portable contract, model specs, and errors.
- `ai-models-core` owns the shared capability check. It validates each
  control against the model spec and applies the fixed values of the spec.
- Each provider crate owns the model specs of its provider. Each adapter
  owns native field names, endpoint selection, and request lifecycle.
- `json-http` carries overall and idle timeouts plus a typed body but never
  inspects provider payloads.

Provider identity remains necessary only at the configuration and composition
boundary that selects an adapter and supplies its credentials. Feature,
evaluation, and execution-policy code receives `dyn Model` plus
`ModelRequest`; it does not inspect provider ids, provider URLs, native field
names, or lifecycle endpoints.

No consumer may infer a provider from a URL, mutate serialized provider JSON,
or choose a completion lifecycle by provider name.

## Shared Contract

`ModelRequest::controls` contains:

- generation controls: optional `effort`, optional `temperature`, optional
  `top_p`, optional `max_output_tokens`, ordered `stop_sequences`, and
  optional `tool_choice`;
- execution controls: optional `total_timeout` and `completion_mode`.

The effort is one of the portable efforts of the
[model capabilities protocol](model-capabilities.md#effort).

Tool choice is one of `None`, `Auto`, `Required`, `RequiredOrAuto`, or
`Function(name)`:

- `Required` strictly requires provider-enforced tool use. If the selected
  provider/model cannot force tool use, the adapter returns
  `UnsupportedControl` before transport.
- `RequiredOrAuto` asks the adapter to force tool use when supported and
  otherwise preserve the tools while permitting automatic selection. The
  evaluation layer remains responsible for verifying that a tool was called.
- `Function(name)` remains strict and has no automatic fallback.

Completion mode is one of:

- `Synchronous`: use the adapter's ordinary immediate lifecycle;
- `PreferDeferred`: use a native deferred lifecycle when the adapter supports
  one, otherwise use its ordinary immediate lifecycle;
- `RequireDeferred`: require a deferred lifecycle or return
  `ModelError::UnsupportedControl`.

All fields default to absence and `Synchronous`. An entirely default controls
object is omitted when `ModelRequest` is serialized. A request without an
effort uses the default effort of the model spec.

`total_timeout` is the total duration available to one adapter invocation. A
streaming immediate adapter uses it as the overall stream deadline without
changing its idle timeout. A deferred adapter applies it across submission,
sleeps, and every retrieval attempt. Retry wrappers remain separate calls and
may apply their own overall policy.

An explicit output limit composes with the max output of the model spec by
using the smaller value. Ordered stop sequences retain caller order.

## Effort Resolution

The [model capabilities protocol](model-capabilities.md#effort) defines how
the capability check resolves the request effort against the model spec. The
resolved effort drives provider serialization.

## Error And Fixed-Value Contract

If a provider cannot honor explicit caller intent, the adapter returns
`ModelError::UnsupportedControl` with the provider, model id, and typed
`ModelControl`. It must fail before transport.

A provider-fixed control may be deliberately ignored only where the provider
defines the value rather than caller intent. The fixed behavior must appear in
the [control matrix](model-capabilities.md#control-matrix) and have a
final-request regression test. This exception applies to sampling on
reasoning models that require native sampling defaults; it does not permit
silently dropping unsupported stops, limits, or forced tool choices.

Each chat adapter holds one model spec. `new`, `with_auth`, and
`with_catalog_auth` take the spec, except that the DeepSeek, Kimi, and Qwen
`new` and `with_auth` build the default spec of their provider. The spec
carries the sampling rule of its model, so a model with adaptive thinking
omits a caller temperature while thinking is on. A model that no catalog
lists uses a [custom spec](model-capabilities.md#custom-specs). The
Anthropic custom defaults keep thinking off unless the request sets an
effort.

Malformed provider responses, terminal provider failures, and transport
failures retain the existing typed `ModelError` classification.

## Provider Matrix

The [control matrix](model-capabilities.md#control-matrix) lists the control
rules of every model spec. The
[capability check](model-capabilities.md#capability-check) applies them
before transport. The
[Chat Completions dialects protocol](chat-completions-dialects.md) and the
[DeepSeek](deepseek-model-provider.md), [Kimi](kimi-model-provider.md),
[Qwen](qwen-model-provider.md), and [MiniMax](minimax-model-provider.md)
provider protocols give the wire mappings of the compatible providers.

These adapters send resolved output limits and stop sequences in these
native fields. The control matrix above defines model support and limits.

| Provider | Output-limit field | Stop field |
| --- | --- | --- |
| OpenAI Responses | `max_output_tokens` | Unsupported |
| Anthropic Messages | `max_tokens` | `stop_sequences` |
| Google Gemini | `generationConfig.maxOutputTokens` | `generationConfig.stopSequences` |
| xAI Chat Completions | `max_tokens` | `stop` |

## System Instructions

- Blank means `system_prompt.trim().is_empty()`: empty and whitespace-only
  prompts are omitted, while nonblank authored text is preserved exactly.
- OpenAI and Anthropic omit blank top-level system instructions.
- Google omits `systemInstruction` when blank.
- Kimi, Qwen, DeepSeek, MiniMax, and XAI omit a blank leading `system`
  message.
- Every nonblank authored instruction is preserved exactly unless structured
  output deliberately appends its documented instruction.
- Empty user, assistant, and tool messages retain each adapter's existing
  role-specific behavior; this rule concerns only synthesized system input.

## Google JSON Schema

Google function declarations serialize the complete caller schema through
`parametersJsonSchema`. They never pass that schema through the restricted
OpenAPI `parameters` field. Structured response schemas remain separate in
`generationConfig.responseJsonSchema`. Constraints such as `uniqueItems` must
reach the final transport request unchanged.

## XAI Deferred Lifecycle

When deferred completion is selected, the XAI adapter:

1. submits exactly once with `deferred: true` and no synchronous streaming
   fields;
2. parses and validates the accepted `request_id` before path construction;
3. polls the deferred-completion endpoint using the same id and fresh
   authenticated request builders;
4. treats `202`, `429`, transient transport failures, and `5xx` retrieval
   responses as retryable retrieval states;
5. never converts a retrieval retry into another completion submission;
6. bounds each poll by the smaller of the remaining total duration and the
   adapter's per-poll timeout;
7. on `200`, maps the completed Chat Completions body to every event of the
   call, as the [model stream protocol](model-stream.md#deadlines) says;
8. surfaces other terminal responses through existing typed status
   classification and reports deadline exhaustion as a transient provider
   timeout.

Polling uses injected clock and sleeper traits in tests. Tests cover success,
pending, rate limiting, transport and server retries, terminal failure,
malformed ids, total timeout, auth on every request, and one submission.

## Wrapper Contract

Retry, concurrency, pricing, multi/fallback, structured-output, mocks, and
tool-calling code pass the complete `ModelRequest` unchanged. Logger copies may
redact private provider replay context but must retain controls. Wrappers do
not reinterpret generation or execution controls.

## Firna Migration

Firna maps fixture and routed policy into `ModelRequest::controls`, uses
`RequiredOrAuto` for benchmarks that prefer forced tool use but permit
provider-native automatic selection, removes its task-local controls and raw
JSON mutator, and removes its XAI transport. A
provider-direct benchmark can use one `PreferDeferred` mode and one total
timeout for every provider; the adapter determines whether the native call is
immediate or deferred. Firna continues to own model ids, credentials, routing,
fallback, pricing, retry, concurrency, and effective routed token limits.

## Verification

Provider tests inspect the final request received by a mocked
`JsonHttpTransport`. No control-mapping or deferred-lifecycle test uses live
credentials. Required regressions cover absent controls, supported controls,
strict and fallback tool choice, retained fallback tools, unsupported errors
before transport, empty/whitespace/nonblank system instructions, wrapper
preservation, full Google schemas, and exactly-once XAI submission. The
credentialed MiniMax suite additionally sends a real MiniMax-M3 tool with
strict `Required` and asserts that the response contains the tool call.

## Official References

- [OpenAI Responses API](https://platform.openai.com/docs/api-reference/responses)
- [Anthropic Messages API](https://platform.claude.com/docs/en/api/messages/create)
- [Anthropic extended thinking](https://platform.claude.com/docs/en/build-with-claude/extended-thinking)
- [Google GenerateContent API](https://ai.google.dev/api/generate-content)
- [Kimi Chat API](https://platform.kimi.ai/docs/api/chat)
- [QwenCloud Chat API](https://docs.qwencloud.com/api-reference/chat/openai-chat)
- [DeepSeek Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion)
- [DeepSeek thinking mode](https://api-docs.deepseek.com/guides/thinking_mode)
- [MiniMax OpenAI-compatible Chat API](https://platform.minimax.io/docs/api-reference/text-chat-openai)
- [XAI deferred completions](https://docs.x.ai/developers/advanced-api-usage/deferred-chat-completions)
