# Provider Call Controls Protocol

## Purpose

Define portable generation and execution intent for every
`ai_interface::Model` call while leaving provider wire formats and completion
lifecycles inside their owning `ai-models-*` adapters.

## Ownership

- Callers choose model, routing, retry, concurrency, pricing, generation
  intent, completion preference, and total deadline.
- `ai-interface` owns the typed portable contract and errors.
- Each provider adapter owns validation, fixed-value behavior, native field
  names, endpoint selection, and request lifecycle.
- `json-http` carries a timeout and typed body but never inspects provider
  payloads.

Provider identity remains necessary only at the configuration and composition
boundary that selects an adapter and supplies its credentials. Feature,
evaluation, and execution-policy code receives `dyn Model` plus
`ModelRequest`; it does not inspect `ProviderKind`, provider URLs, native field
names, or lifecycle endpoints.

No consumer may infer a provider from a URL, mutate serialized provider JSON,
or choose a completion lifecycle by provider name.

## Shared Contract

`ModelRequest::controls` contains:

- generation controls: optional `temperature`, optional `top_p`, optional
  `max_output_tokens`, ordered `stop_sequences`, and optional `tool_choice`;
- execution controls: optional `total_timeout` and `completion_mode`.

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
object is omitted when `ModelRequest` is serialized. Existing callers
therefore retain provider-native defaults.

`total_timeout` is the total duration available to one adapter invocation. An
immediate adapter applies it to its HTTP request. A deferred adapter applies it
across submission, sleeps, and every retrieval attempt. Retry wrappers remain
separate calls and may apply their own overall policy.

An explicit output limit composes with a stricter adapter-native maximum by
using the smaller value. Ordered stop sequences retain caller order.

## Error And Fixed-Value Contract

If a provider cannot honor explicit caller intent, the adapter returns
`ModelError::UnsupportedControl` with the provider, model id, and typed
`ModelControl`. It must fail before transport.

A provider-fixed control may be deliberately ignored only where the provider
defines the value rather than caller intent. The fixed behavior must appear in
the matrix below and have a final-request regression test. This exception
applies to sampling on reasoning models that require native sampling defaults;
it does not permit silently dropping unsupported stops, limits, or forced tool
choices.

Malformed provider responses, terminal provider failures, and transport
failures retain the existing typed `ModelError` classification.

## Provider Matrix

`map` means serialize the native field. `fixed` means omit the field and
preserve the provider/model value even when requested. `unsupported` means an
explicit request returns `UnsupportedControl` before transport. Empty controls
always preserve the adapter's prior behavior.

| Provider and mode | Temperature / top-p | Output limit | Stops | Tool choice |
| --- | --- | --- | --- | --- |
| OpenAI Responses, no reasoning | map | `max_output_tokens` | unsupported | map all |
| OpenAI Responses, reasoning | fixed | `max_output_tokens` | unsupported | map all |
| Anthropic, no thinking | map | `min(4096, requested)` | `stop_sequences` | map all |
| Anthropic, thinking | fixed | `min(4096, requested)` | `stop_sequences` | map all |
| Google Gemini | map | `maxOutputTokens` | `stopSequences` | map all |
| Kimi K3 | fixed | `max_completion_tokens` | `stop` | map all |
| Qwen, no thinking | map | `max_completion_tokens` | `stop` | map all; `RequiredOrAuto` -> required |
| Qwen, thinking | fixed | `max_completion_tokens` | `stop` | none/auto; strict forced unsupported; `RequiredOrAuto` -> auto |
| DeepSeek, no thinking | map | `max_tokens` | `stop` | map all |
| DeepSeek, thinking | fixed | `max_tokens` | `stop` | auto by omission; strict forced unsupported; `RequiredOrAuto` -> omission |
| MiniMax-M3 | map | `max_completion_tokens` | unsupported | none, auto, required; `RequiredOrAuto` -> required |
| Other MiniMax catalog models | map | `max_completion_tokens` | unsupported | none/auto; required unsupported; `RequiredOrAuto` -> auto |
| XAI Chat Completions | map | `max_tokens` | `stop` | map all |

For OpenAI, Anthropic, Google, Kimi, and XAI, `RequiredOrAuto` maps to the
same provider-enforced form as `Required`. Tool definitions remain serialized
in every `RequiredOrAuto` fallback path.

For every non-XAI adapter, `PreferDeferred` uses the ordinary immediate call
and `RequireDeferred` is unsupported. XAI maps either deferred mode to its
native deferred lifecycle.

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

1. submits exactly once with `deferred: true`;
2. parses and validates the accepted `request_id` before path construction;
3. polls the deferred-completion endpoint using the same id and fresh
   authenticated request builders;
4. treats `202`, `429`, transient transport failures, and `5xx` retrieval
   responses as retryable retrieval states;
5. never converts a retrieval retry into another completion submission;
6. bounds each poll by the smaller of the remaining total duration and the
   adapter's per-poll timeout;
7. returns the completed Chat Completions body on `200`;
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
