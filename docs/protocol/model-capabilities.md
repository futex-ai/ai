# Model Capabilities Protocol

Status: Approved, not implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

When implemented, this protocol replaces the thinking-level resolution
section and the provider matrix of
[provider-call-controls.md](provider-call-controls.md).

This protocol defines model specs, provider ids, text ids, custom specs,
effort, the capability check, and the control matrix. The portable controls
themselves stay in [provider call controls](provider-call-controls.md).

## Model specs

`ModelSpec` holds the facts of one model. The protocol docs call a model spec
a spec for short.

| Field | Meaning |
| --- | --- |
| Provider id | The `ProviderId` of the provider that serves the model |
| Provider model id | The model id that the adapter sends to the provider |
| Context window | The total context tokens that the provider documents, or 0 when unknown |
| Max output | The largest output-token limit that the provider accepts for the model |
| Efforts | The ordered set of efforts that the model supports. It is never empty |
| Default effort | The effort of a request that sets none. It is one of the efforts |
| Forced tool choice | The forced-tool-choice rule of the model |
| Sampling | Adjustable, fixed while thinking is on, or always fixed |
| Stop sequences | Supported or not supported |
| Deferred lifecycle | Whether the adapter has a deferred lifecycle for the model |
| Replay need | Whether a tool continuation needs the provider state of the previous assistant message, for each effort |
| Features | The `ModelFeature` values that routing reads |
| Routing tiers | An intelligence score from 1 to 10, a speed tier, and a cost tier |

- The speed tiers are slow, medium, fast, and very fast. The cost tiers are
  low, medium, high, and premium.
- The routing tiers describe the model at its default effort.
- The replay need is data for callers and for the test kit. The conformance
  runner checks replay for each spec that has the replay need.
- Model specs hold no prices. Decision: prices come from the deployment, as
  [model policies](model-policies.md) says, because deployment prices
  change without a release. The rejected alternative put prices in specs.

## Catalogs

- Each provider crate exports one constant for each spec and a `CATALOG`
  slice of its specs.
- A catalog lists each provider model id once.
- `catalog()` combines the catalogs of the providers that the registry
  includes.
- Decision: the request carries the effort, and a catalog lists one spec for
  each provider model. The rejected alternative was one catalog entry for
  each thinking level, such as `claude-opus-5-thinking-max`. That repeated
  the facts of a model in each entry and fixed the effort when the host
  built the model.

## Provider ids

- A provider id is a lowercase string. It never changes, because stored
  provider state and host configs hold it.
- The provider ids are `anthropic`, `deepseek`, `google`, `kimi`, `minimax`,
  `mock`, `openai`, `qwen`, `typesafe`, and `xai`.
- Each provider crate defines the `ProviderId` constant of its provider. The
  contracts define only the `mock` id of their test model.
- Decision: provider ids are constants of the provider crates. The rejected
  alternative was one enum of every provider in the contracts, which made
  the contracts name every provider.

## Text ids

- A text id has the form `provider/model`: the provider id, a slash, and
  the provider model id.
- The provider id ends at the first slash. The model part can hold more
  slashes.
- Text ids are case-sensitive.
- `catalog().find(text_id)` returns the spec with that text id, or none.
- `find` reads only the catalogs. It sends no request.
- Hosts resolve text ids to specs when they load their configuration. A
  built model holds its spec and never resolves a text id again.
- A routing requirement names a model by its text id and a provider by its
  provider id.

## Custom specs

- `ModelSpec::custom` builds a spec from a provider id and a provider model
  id for a model that no catalog lists.
- The caller can set every other field of a custom spec.
- Each field that the caller does not set takes the custom default of the
  provider.
- The custom defaults of a provider give a max output when the provider
  requires an output limit on every request.
- A custom spec has no features and no routing tiers until the caller sets
  them.
- A custom spec with a provider id that the registry does not include fails
  when the registry builds it, as the
  [provider registry protocol](provider-registry.md) says.

## Effort

- `Effort` has these values, in order: disabled, low, medium, high, extra
  high, and max.
- A request can set an effort in its generation controls.
- Thinking is on when the resolved effort is not disabled.
- A request without an effort uses the default effort of the spec.
- The capability check lowers a requested effort that the spec does not
  list to the highest listed effort below it.
- The check never raises an effort. When no listed effort is at or below the
  request, the call fails with the unsupported control kind for effort.
- The finish event and the response record the resolved effort.
- Each adapter maps each effort to the wire fields of its provider.
- Decision: the check lowers and never raises. The rejected alternative
  raised to the nearest effort, which can add cost that the caller did not
  ask for.

## Capability check

Every adapter applies the capability check before transport, with the same
rules for every provider. The check reads the request and the spec. It
either fails with the unsupported control kind or gives the resolved
controls. The adapter sends exactly the resolved controls.

| Control | Rule |
| --- | --- |
| Effort | The effort rules above. |
| Output limit | A limit above the max output becomes the max output. A request without a limit keeps none. |
| Temperature and top-p | Sent when sampling is adjustable. Left out, also when requested, while sampling is fixed. |
| Stop sequences | A non-empty list fails when the spec does not support stop sequences. |
| Tool choice | The tool-choice rules below. |
| Completion mode | Synchronous uses the synchronous lifecycle. Prefer deferred uses the deferred lifecycle when the spec has one, and the synchronous lifecycle otherwise. Require deferred fails when the spec has none. |

- An adapter whose provider requires an output limit sends the max output
  when the request has no limit.
- Decision: the max output of each spec is the ceiling of every request and
  the default limit where the provider requires a limit. The rejected
  alternatives were these:
  - A fixed 4,096-token ceiling for every Anthropic model, which cuts long
    answers short.
  - A larger constant, which some models reject.
  - No ceiling, which lets an old model reject a large default.
- An unsupported control error names the control, the provider id, and the
  provider model id.
- A failed check sends no HTTP request.
- Decision: one shared check holds every control rule as spec data. The
  rejected alternative let each adapter check its own controls, so that a
  rule such as the Anthropic tool-choice rule lived in one adapter only.

## Tool choice

| Forced tool choice rule | Required | Function | Examples |
| --- | --- | --- | --- |
| Always | allowed | allowed | OpenAI, Google, Kimi, xAI |
| Only without thinking | allowed while thinking is off | allowed while thinking is off | Anthropic, Qwen, DeepSeek |
| Required only | allowed | not allowed | MiniMax-M3 |
| Never | not allowed | not allowed | the other MiniMax models |

- None and auto are allowed for every spec.
- A required or function choice that the rule does not allow fails with the
  unsupported control kind for tool choice.
- Required-or-auto becomes required when the rule allows required, and auto
  otherwise.
- The check never removes a tool definition. A required-or-auto choice that
  becomes auto keeps every tool.
- When thinking is on for an Anthropic model, required and function fail
  with the unsupported control kind before transport, and required-or-auto
  is sent as auto. Decision: the provider rejects a forced tool choice while
  thinking is on. The rejected alternative turned thinking off when a tool
  was forced, which changes the model behavior without telling the caller.

## Control matrix

`cargo xtask capability-matrix` generates this section from the specs, and
the generated section is committed. Do not edit it by hand.

- The matrix has one row for each spec, and one `custom` row for each
  provider that shows its custom defaults.
- Its columns are these:
  - The text id.
  - The efforts, with the default effort marked.
  - The max output and the context window.
  - Sampling and stop sequences.
  - The forced tool-choice rule.
  - The deferred lifecycle and the replay need.
- Decision: no CI step compares the committed matrix with the generator
  output. The rejected alternative was such a step, which is a new gate.

Milestone 5 of the plan generates the first content of this section.
