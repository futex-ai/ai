# Conversation Parts Protocol

Status: Approved, not implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

When implemented, this protocol replaces the shared boundary section of
[video-input.md](video-input.md) and the replay items in `ai-interface` that
the DeepSeek, Kimi, Qwen, and MiniMax provider docs define.

This protocol defines messages, conversation parts, provider state, the
replay and skip rules of adapters, the stored message format, the converter
for the old format, and log copies.

## Terms

- **Part:** one conversation part of a message.
- **Replay data:** a value that a provider requires back on a later
  request, such as a reasoning signature.
- **Own state:** provider state whose provider id is the provider id of the
  adapter that reads it.

## Messages

- A message has a role, ordered parts, and an optional participant name.
- The roles are user, assistant, and tool.
- Messages have no system role. The system prompt of the request carries the
  system instructions. Decision: the request keeps its system prompt, so
  that one place applies the blank-system rule of
  [provider call controls](provider-call-controls.md). The rejected
  alternative was a system role on messages.

| Role | Parts that the role allows |
| --- | --- |
| User | text, image, video |
| Assistant | text, reasoning, tool call |
| Tool | exactly one tool result |

- An adapter fails a request that breaks this table with the invalid request
  kind before transport.
- A user message can have no parts. An adapter then sends empty content.
- An adapter sends the participant name of a user or assistant message only
  when its provider takes names.

## Parts

| Part | `type` | Fields |
| --- | --- | --- |
| Text | `text` | `text` |
| Image | `image` | `mime_type`, `data_base64` |
| Video | `video` | `mime_type`, `data_base64` |
| Tool call | `tool_call` | `call_id`, `tool_name`, `arguments`, optional `operation_id` |
| Tool result | `tool_result` | `call_id`, `tool_name`, `content` |
| Reasoning | `reasoning` | `text` |

- The `type` column and the field names are the JSON names of the stored
  format.
- Every part can carry at most one provider state, in its optional
  `provider_state` field.
- The arguments of a tool call are one JSON value.
- The operation id of a tool call is the idempotency key that the agent loop
  gives to the tool.
- The content of a tool result is text.
- The call id of a tool result names the tool call that the result answers.
- The text of a reasoning part is the reasoning that the provider exposed.
  It is empty when the provider exposed only replay data.
- The contracts do not check MIME types or sizes. The
  [video input protocol](video-input.md) and the provider docs give the
  media rules of each provider.

## Provider state

`ProviderState { provider, model, version, payload }` carries replay data:

| Field | Meaning |
| --- | --- |
| `provider` | The provider id of the adapter that wrote the state |
| `model` | The provider model id that produced the state, or empty for state that the converter wrote |
| `version` | The format version of the payload, a positive integer that the adapter defines |
| `payload` | A JSON object that only the writing adapter reads |

- Provider state is opaque outside the adapter that wrote it. No other
  component reads or changes the payload.
- An adapter records as provider state every value that its provider
  requires on a later request.
- Anthropic thinking blocks with their signatures, Anthropic
  redacted-thinking blocks, and Gemini thought signatures are replay data.
- An adapter attaches state only to the parts that it produces.
- Decision: replay data travels as opaque provider state on parts, with a
  JSON object payload and a format version. Hosts store conversations as
  JSON, and each adapter types its own payload. The rejected alternative was
  a closed set of typed provider items in the contracts, which made the
  contracts name every provider.

## Replay and skip rules

- An adapter sends the messages of a request in order, and the parts of a
  message in order.
- An adapter replays all its own state, from any model of its provider.
- An adapter ignores the provider state of every other provider.
- An adapter still sends a text part or a tool-call part that carries the
  state of another provider. It sends the part in its portable form.
- An adapter skips a reasoning part that carries no own state.
- An adapter replays a reasoning part that carries own state, in the form
  that its provider requires.
- An adapter decodes its own state before transport.
- Own state with an unknown version, or with a payload that does not decode,
  fails the call with the invalid request kind before transport. Decision:
  the call fails, because a provider rejects or misreads a tool
  continuation without its replay data. The rejected alternative skipped
  such state.
- An adapter fails a part that its model does not take with the unsupported
  input kind before transport.

## Stored format

- A serialized message records its format version.
- Format version 2 is the format of this protocol.
- A serialized message without a format version has format version 1.
  Version 1 has the fields `content`, `content_parts`, `name`,
  `tool_call_id`, `tool_calls`, and `provider_context`.
- Reading accepts versions 1 and 2. Reading a version 1 message applies the
  converter that the registry exposes.
- Writing produces version 2 only.
- Reading a message with another version fails with a typed error.
- A version 2 message has the fields `format_version`, `role`, `parts`, and
  an optional `name`. The parts table gives the fields of each part.

This version 2 assistant message shows the shape. The payload is an example
only:

```json
{
  "format_version": 2,
  "role": "assistant",
  "parts": [
    {
      "type": "reasoning",
      "text": "The user asks for a search.",
      "provider_state": {
        "provider": "anthropic",
        "model": "claude-opus-5",
        "version": 1,
        "payload": { "signature": "EqQBCkYIBRgCIkD..." }
      }
    },
    { "type": "text", "text": "I will search for it." },
    {
      "type": "tool_call",
      "call_id": "toolu_01",
      "tool_name": "search",
      "arguments": { "query": "rust" }
    }
  ]
}
```

## Converter

The converter reads a version 1 message and returns a version 2 message.
It is one-way: no function writes version 1.

- Decision: a one-way converter reads the old format, so that stored
  conversations survive. The rejected alternative dropped the old format,
  which loses every stored conversation.
- The registry exposes the converter that hosts call and composes it, as
  the [provider registry protocol](provider-registry.md) says.
- Each provider crate converts the version 1 items of its own provider. It
  writes their provider state, and it orders the parts of their message
  with its own order rule.
- Decision: each provider crate converts its own items, so that a converted
  message gives the provider of its items the same request as the
  version 1 message. The registry composes the conversions, because only
  the registry names every provider. The rejected alternative was one order
  rule for every provider. Parts grouped by kind change the OpenAI request,
  and parts in the order of `provider_context` change the Google request.
- The role does not change.
- The `name` of a user or assistant message becomes its participant name.
- A user message with `content_parts` gets those parts in order. A user
  message without them gets one text part from a non-empty `content`.
- A tool message gets one tool result part. Its call id is `tool_call_id`,
  its tool name is `name`, and its content is `content`.
- An assistant message gets one text part from a non-empty `content`, one
  tool-call part for each entry of `tool_calls`, and the new reasoning
  parts that the table below gives. The order rules below give the order
  of these parts.
- A tool-call part keeps the id, the name, the input, and the operation id
  of its `tool_calls` entry.
- When an assistant or tool message has `content_parts`, the converter
  drops them and writes a `tracing` warning.
- Each provider item becomes provider state with the provider id of its
  provider crate, version 1, an empty model, and the fields of the item as
  the payload.
- Version 1 of each provider payload is the shape of its old provider item.
  Each adapter reads version 1.

| Old item | Provider | Part that carries the state |
| --- | --- | --- |
| `openai_message` | `openai` | The text part, for the first `openai_message` item only |
| `openai_reasoning` | `openai` | A new reasoning part with empty text |
| `openai_function_call` | `openai` | The tool-call part whose call id is the item's `call_id` |
| `minimax_assistant` | `minimax` | A new reasoning part with the item's `reasoning_content` as text |
| `xai_legacy_function_call` | `xai` | The tool-call part whose call id is the item's `tool_call_id` |
| `deep_seek_assistant_message` | `deepseek` | A new reasoning part with the item's `reasoning_content` as text |
| `kimi_assistant_message` | `kimi` | A new reasoning part with the item's `reasoning_content` as text |
| `qwen_assistant_message` | `qwen` | A new reasoning part with the item's `reasoning_content` as text |

- An Anthropic thinking item or a Gemini thought-signature item follows the
  same rule. An item for reasoning becomes a new reasoning part, and an item
  for one tool call attaches to that tool-call part.
- When the part for an item does not exist, the converter drops the item and
  writes a `tracing` warning.
- When the registry does not include the provider of an item, the converter
  keeps the item as provider state of that provider, with version 1 and the
  fields of the item as the payload. The state goes on the part that the
  table gives, or on a new reasoning part with empty text when that part
  does not exist.
- When one message holds items of more than one provider, the provider of
  the first item sets the order of the parts. The items of the other
  providers attach as their provider state, as the table says.
- Decision: the converter keeps the items of a provider that the registry
  does not include. The rejected alternatives dropped those items, which
  loses replay data for good, or failed the read, which adds a rejection.

These order rules give the order of the parts of an assistant message:

- A message without provider items gets the text part, then the tool-call
  parts in the order of `tool_calls`.
- The OpenAI conversion keeps the order of `provider_context`:
  - Each `openai_reasoning` item gives a reasoning part at its position.
  - The text part takes the position of the first `openai_message` item.
  - Each tool-call part takes the position of its `openai_function_call`
    item.
  - A text part without an `openai_message` item, and the tool-call parts
    without an `openai_function_call` item, come after the parts of the
    items. The text part comes first.
- The OpenAI text part takes the phase of the first `openai_message` item.
  Later `openai_message` items add nothing.
- Every other provider conversion gives the reasoning parts of its items
  first, in the order of `provider_context`. Then comes the text part, then
  the tool-call parts in the order of `tool_calls`. For Google, the text
  thus comes before the function calls.

## Log copies

- The contracts give a log copy of a request, of a message, and of a
  response.
- A log copy keeps every message and every part.
- In a log copy, no part carries provider state, and every reasoning part
  has empty text.
- The request that an adapter sends and the conversation of the caller keep
  their provider state.
- The agent loop gives observers log copies only, as the
  [agent loop protocol](agent-loop.md) says.
- Decision: one redaction rule covers every provider. The rejected
  alternative listed the replay items to remove, and a new provider item
  could then reach the logs.
