# Architecture

Status: Implemented.

This document describes the structure of the workspace. It names the
components, the responsibility of each component, the boundaries between
them, and the flow of data in one model call and in one agent turn. The
protocol docs under [`protocol/`](protocol/) hold the requirements at each
boundary.

## Components

| Component | Crate | Responsibility |
| --- | --- | --- |
| Contracts | `ai-interface` | The traits and the data that cross every other boundary, and the fold from model events to a response. |
| HTTP | `json-http` | Requests, Server-Sent Events (SSE) streams, response headers, typed bodies, and record-and-replay for tests. |
| Adapter helpers | `ai-models-core` | Shared adapter logic: the capability check, error classification, schema checks, and synthetic tool-call ids. |
| Native adapters | `ai-models-anthropic`, `ai-models-openai`, `ai-models-google`, `ai-models-typesafe` | One adapter for each native provider protocol. |
| Compatible engine | `ai-models-compat` | One Chat Completions engine, with one dialect each for DeepSeek, Kimi, Qwen, MiniMax, and xAI. |
| Policies | `ai-models-policy` | The retry, fallback, concurrency, and pricing layers. |
| Registry | `ai-models-registry` | Provider configs, the combined catalog, one factory for every model spec, and the converter of the old stored format. |
| Agent loop | `ai-tool-calling` | Turns over a caller-owned conversation, parallel tool calls, outcomes, observers, and tool output windows. |
| MCP | `ai-mcp`, `ai-mcp-oauth` | The Model Context Protocol (MCP) client, its tool adapter, and its OAuth flow. |
| Test kit | `ai-testkit` | Scenarios, provider stream recordings, and the conformance runner. |

### Contracts

The contracts define `Model` and its event stream, `Tool`, and the traits of
the other modalities: image generation, video generation, audio
transcription, and judgment. They also define messages and conversation
parts, provider state, model specs, efforts, provider ids, usage, and the
error core with its error kinds. The contracts name no provider. A provider
id is a value that the provider crate defines. The contracts do no I/O.

The contracts also hold the fold from model events to a response, beside
`Model`. The provided `Model::complete` and the agent loop share this one
fold. The contracts depend on no other component, so the fold must live in
the contracts for `Model::complete` to use it without a dependency cycle.

### HTTP

The HTTP component sends requests and opens SSE streams through an injected
transport. It applies auth hooks and headers, and it enforces the idle and
overall deadlines of a stream. It gives its callers the status, the response
headers, and a typed body. It never reads a provider payload.

### Adapter helpers

The adapter helpers hold the logic that every adapter shares. They check
each request against its model spec. They map HTTP and stream failures to
error kinds, check tool-call arguments and structured output, and make
synthetic tool-call ids. They name no provider and do no I/O of their own.

### Native adapters and the compatible engine

An adapter maps the shared request and the conversation parts to the wire
format of its provider. It replays its own provider state and skips the
state of other providers. It maps the provider stream to model events, and
it maps provider failures to error kinds. Each provider crate exports its
provider id and the model specs of its catalog. It also exports the
conversion of the replay items of its own provider in the old stored
format, as the [conversation parts protocol](protocol/conversation-parts.md)
says.

The compatible engine is one adapter for the Chat Completions protocol. The
engine owns all the behavior that the five Chat Completions providers share.
A dialect sets only the values that differ for one provider. The
[dialect protocol](protocol/chat-completions-dialects.md) defines the
boundary between the engine and a dialect.

### Policies

A policy layer wraps any `Model` and is itself a `Model`. It passes the
request on unchanged, and it keeps the order of the model events. A policy
layer decides from error kinds only. It never decides from a provider id or
from error text.

### Registry

The registry is the one component that names every provider. It holds one
provider config for each provider. It builds the adapter of each modality
from a model spec, and it combines the catalogs of all providers. It also
composes the conversions of its providers into the one converter of the old
stored format that hosts call. One Cargo feature for each provider selects
the providers that the registry includes. The other library components do
not depend on the registry.

### Agent loop

The agent loop runs one turn over a conversation that the caller owns. It
streams each model call and builds the response of each step with the fold
of the contracts, the same fold that `Model::complete` uses. It runs tool
calls in parallel up to a limit, bounds tool output, and reports the
outcome. It knows `Model` and `Tool` only. It does not know which provider
serves a model or which policy layers wrap it.

### MCP

The MCP client exposes the tools of a remote MCP server as `Tool` values,
and the OAuth companion gets and refreshes the credentials of that server.
The MCP crates send their requests through their own HTTP transports.
They use only the auth hook interface of the HTTP component. The open
follow-up item
[Choose how the MCP crates change](../plans/follow-ups/mcp-crate-direction.md)
decides where the MCP protocol logic and its HTTP code live.

### Test kit

The test kit replays recorded provider streams through each adapter. Its
conformance runner checks the order of the model events and snapshots the
results. No product code depends on the test kit.

## Dependency rules

- The contracts and the HTTP component depend on no other workspace
  component.
- The adapter helpers depend on the contracts and the HTTP component.
- The native adapters and the compatible engine depend on the contracts,
  the HTTP component, and the adapter helpers.
- The policies and the agent loop depend on the contracts. They never
  depend on an adapter.
- The registry depends on the contracts, the HTTP component, the adapter
  helpers, and each provider crate that a Cargo feature selects.
- The MCP crates depend on the contracts and the HTTP component.
- The test kit can depend on every other component.

## Boundaries

- Only an adapter knows a provider wire format.
- Only the registry names every provider.
- Provider state is opaque outside the adapter that wrote it.
- The HTTP component never reads a provider payload.
- Policy layers know error kinds, not providers.
- The agent loop changes the caller's conversation only by appending
  messages.
- Prices come from the deployment. Model specs hold no prices.

## Flow of one model call

1. The host turns a text id, such as `anthropic/claude-opus-5`, into a model
   spec through the combined catalog.
2. The registry builds the adapter for the spec from the provider config of
   the spec's provider.
3. The host wraps the adapter in policy layers. A typical stack is fallback
   over models that each have retry, concurrency, and pricing layers.
4. The caller calls `Model::stream` with a request. `Model::complete` calls
   the same method and folds the events.
5. The adapter runs the capability check against its spec. The check either
   fails with an unsupported control before transport, or it resolves the
   effort, the output limit, and the tool choice.
6. The adapter maps the request and the conversation parts to its wire
   format. The HTTP component opens the provider stream.
7. The adapter maps each provider event to model events. It records the
   replay data of its provider as provider state on the parts.
8. The policy layers forward the events. The pricing layer prices the usage
   event. The fallback layer reports each failed attempt with a fallback
   event.
9. The caller reads the events until the finish event, or the fold returns
   the response.

## Flow of one agent turn

1. The caller appends a user message to its conversation. It calls `run`
   with the conversation and an agent request.
2. Each step builds a model request from the system prompt, the
   conversation, the tool definitions, and the controls. The loop forwards
   each model event as an agent event.
3. The loop builds the response of the step with the fold of the
   contracts, the same fold that `Model::complete` uses. Then it calls the
   response checkpoint.
4. The loop appends the assistant message to the conversation.
5. When the finish reason is tool calls, the loop runs the calls in
   parallel up to its limit. It bounds each output with the output store.
   It appends the tool results in call order as the calls complete. Then
   the next step starts.
6. The turn ends with an outcome: completed, truncated, or capped. A model
   error or a checkpoint error ends it with that error. A stopped turn keeps
   every completed tool result, so its final step can be incomplete.
7. Observers receive log copies during the turn. Their errors go to
   `tracing`.

## Related docs

- [Model stream](protocol/model-stream.md)
- [Conversation parts](protocol/conversation-parts.md)
- [Model capabilities](protocol/model-capabilities.md)
- [Chat Completions dialects](protocol/chat-completions-dialects.md)
- [Provider registry](protocol/provider-registry.md)
- [Model policies](protocol/model-policies.md)
- [Agent loop](protocol/agent-loop.md)
- [Tool output management](protocol/tool-output-management.md)
