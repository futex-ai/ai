# Model Capabilities Protocol

Status: Implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

This protocol defines model specs, provider ids, text ids, custom specs,
effort, the capability check, and the control matrix. The portable controls
themselves stay in [provider call controls](provider-call-controls.md). That
protocol points here for effort resolution and for the control rules of
each model.

## Model specs

`ModelSpec` holds the facts of one model. The protocol docs call a model spec
a spec for short.

| Field | Meaning |
| --- | --- |
| Provider id | The `ProviderId` of the provider that serves the model |
| Provider model id | The model id that the adapter sends to the provider |
| Context window | The total context tokens that the provider documents, or 0 when unknown |
| Max output | The largest output-token limit that the provider accepts for the model |
| Output limit required | Whether the provider requires an output limit on every request |
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
  runner checks replay for each spec that has this need.
- Model specs hold no prices. Decision: prices come from the deployment, as
  [model policies](model-policies.md) says, because deployment prices
  change without a release. The rejected alternative put prices in specs.

## Catalogs

- Each provider crate exports one constant for each spec and a `CATALOG`
  slice of its specs.
- A catalog lists each provider model id once.
- `ModelCatalog` combines the catalogs of several providers. It keeps the
  specs in the order in which the host adds them.
- The [registry](provider-registry.md) combines the catalogs of the
  providers that it includes.
- Decision: the request carries the effort, and a catalog lists one spec for
  each provider model. The rejected alternative was one catalog entry for
  each pair of a provider model and a thinking level. That repeated the
  facts of a model in each entry and fixed the effort when the host built
  the model.

## Provider ids

- A provider id is a lowercase string. It never changes, because stored
  provider state and host configs hold it.
- The provider ids are `anthropic`, `deepseek`, `google`, `kimi`, `minimax`,
  `mock`, `openai`, `qwen`, `typesafe`, and `xai`.
- Each provider crate defines the `ProviderId` constant of each provider
  that it serves. The contracts define only the `mock` id of their test
  model.
- Decision: provider ids are constants of the provider crates. The rejected
  alternative was one enum of every provider in the contracts, which made
  the contracts name every provider.

## Text ids

- A text id has the form `provider/model`: the provider id, a slash, and
  the provider model id.
- The provider id ends at the first slash. The model part can hold more
  slashes.
- Text ids are case-sensitive.
- `find(text_id)` on a combined catalog returns the spec with that text id,
  or none.
- `find` reads only the catalogs. It sends no request.
- Hosts resolve text ids to specs when they load their configuration. A
  built model holds its spec and never resolves a text id again.
- A routing requirement names a model by its text id and a provider by its
  provider id.

## Custom specs

- Each provider crate exports the custom defaults of each provider that it
  serves.
- `ModelSpec::custom` builds a spec from a provider id, a provider model id,
  and the custom defaults of that provider, for a model that no catalog
  lists.
- The caller can set every other field of a custom spec.
- Each field that the caller does not set takes the custom default of the
  provider.
- The custom defaults of a provider give a max output when the provider
  requires an output limit on every request.
- A custom spec has no features and no routing tiers until the caller sets
  them.
- The [provider registry protocol](provider-registry.md) says how the
  registry treats a custom spec of a provider that it does not include.

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
- The response of an adapter records the resolved effort and the text id of
  its spec. The [model stream protocol](model-stream.md) puts the resolved
  effort on its finish event.
- Each adapter maps each effort to the wire fields of its provider.
- Decision: the check lowers and never raises. The rejected alternative
  raised to the nearest effort, which can add cost that the caller did not
  ask for.

## Capability check

Every `Model` adapter applies the capability check before transport, with
the same rules for every provider. The check reads the request and the
spec. It either fails with the unsupported control kind or gives the
resolved controls. The adapter sends exactly the resolved controls.

| Control | Rule |
| --- | --- |
| Effort | The effort rules above. |
| Output limit | A limit above the max output becomes the max output. A request without a limit keeps none, unless the spec requires an output limit. Then the limit is the max output. |
| Temperature and top-p | Sent when sampling is adjustable. Left out, also when requested, while sampling is fixed. |
| Stop sequences | A non-empty list fails when the spec does not support stop sequences. |
| Tool choice | The tool-choice rules below. |
| Completion mode | Synchronous uses the synchronous lifecycle. Prefer deferred uses the deferred lifecycle when the spec has one, and the synchronous lifecycle otherwise. Require deferred fails when the spec has none. |

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

<!-- capability-matrix:start -->

Bold effort marks the default effort. `default` marks an output limit sent when the request omits one.
`u32::MAX` adds no output ceiling beyond the request type. Context 0 means unknown.

| Text id | Efforts | Max output | Context | Sampling | Stops | Forced tools | Deferred | Replay efforts |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `anthropic/claude-sonnet-5` | low, medium, **high** | 128000 default | 1000000 | fixed while thinking | yes | only without thinking | no | low, medium, high |
| `anthropic/claude-opus-5` | low, medium, **high**, max | 128000 default | 1000000 | fixed while thinking | yes | only without thinking | no | low, medium, high, max |
| `anthropic/claude-fable-5` | low, medium, **high** | 128000 default | 1000000 | fixed while thinking | yes | only without thinking | no | low, medium, high |
| `anthropic/claude-sonnet-4-6` | **disabled** | 128000 default | 200000 | fixed while thinking | yes | only without thinking | no | none |
| `anthropic/claude-opus-4-7` | **disabled**, max | 128000 default | 200000 | fixed while thinking | yes | only without thinking | no | max |
| `anthropic/claude-haiku-4-5` | **disabled** | 64000 default | 200000 | fixed while thinking | yes | only without thinking | no | none |
| `anthropic/custom` | **disabled**, low, medium, high, extra_high, max | 64000 default | 0 | fixed while thinking | yes | only without thinking | no | low, medium, high, extra_high, max |
| `deepseek/deepseek-v4-pro` | disabled, **high**, max | 393216 | 1000000 | fixed while thinking | yes | only without thinking | no | high, max |
| `deepseek/deepseek-v4-flash` | disabled, **high**, max | 393216 | 1000000 | fixed while thinking | yes | only without thinking | no | high, max |
| `deepseek/custom` | disabled, **high**, max | 393216 | 0 | fixed while thinking | yes | only without thinking | no | high, max |
| `google/gemini-3.6-flash` | low, **medium**, high | 65536 | 1048576 | adjustable | yes | always | no | low, medium, high |
| `google/gemini-3.5-flash-lite` | **disabled** | 65536 | 1048576 | adjustable | yes | always | no | disabled |
| `google/gemini-3.1-flash-image` | **disabled** | 0 | 131072 | adjustable | yes | always | no | none |
| `google/custom` | **disabled**, low, medium, high, extra_high, max | u32::MAX | 0 | adjustable | yes | always | no | disabled, low, medium, high, extra_high, max |
| `kimi/kimi-k3` | low, high, **max** | 131072 | 1000000 | fixed | yes | always | no | low, high, max |
| `kimi/custom` | low, high, **max** | 131072 | 0 | fixed | yes | always | no | low, high, max |
| `minimax/MiniMax-M3` | disabled, **medium** | 512000 | 1000000 | adjustable | no | required only | no | medium |
| `minimax/MiniMax-M2.7` | **medium** | 131072 | 204800 | adjustable | no | never | no | medium |
| `minimax/MiniMax-M2.7-highspeed` | **medium** | 131072 | 204800 | adjustable | no | never | no | medium |
| `minimax/custom` | disabled, low, **medium**, high, extra_high, max | u32::MAX | 0 | adjustable | no | never | no | low, medium, high, extra_high, max |
| `mock/mock` | **disabled** | 128000 | 128000 | adjustable | yes | always | no | none |
| `mock/custom` | **disabled** | 128000 | 128000 | adjustable | yes | always | no | none |
| `openai/gpt-5.6-sol` | low, **medium**, high, extra_high, max | 128000 | 1050000 | fixed while thinking | no | always | no | low, medium, high, extra_high, max |
| `openai/gpt-5.6-terra` | low, **medium** | 128000 | 1050000 | fixed while thinking | no | always | no | low, medium |
| `openai/gpt-5.6-luna` | low, **medium** | 128000 | 1050000 | fixed while thinking | no | always | no | low, medium |
| `openai/gpt-5.5` | low, **medium**, high, extra_high | 128000 | 400000 | fixed while thinking | no | always | no | low, medium, high, extra_high |
| `openai/gpt-5.4-mini` | **disabled** | 128000 | 400000 | fixed while thinking | no | always | no | disabled |
| `openai/gpt-5.4-nano` | **disabled** | 128000 | 400000 | fixed while thinking | no | always | no | disabled |
| `openai/gpt-image-2` | **disabled** | 0 | 0 | fixed while thinking | no | always | no | none |
| `openai/custom` | **disabled**, low, medium, high, extra_high, max | u32::MAX | 0 | fixed while thinking | no | always | no | disabled, low, medium, high, extra_high, max |
| `qwen/qwen3.7-max` | disabled, **high** | 65536 | 1000000 | fixed while thinking | yes | only without thinking | no | high |
| `qwen/qwen3.7-plus` | disabled, **high** | 65536 | 1000000 | fixed while thinking | yes | only without thinking | no | high |
| `qwen/qwen3.7-flash` | disabled, **high** | 65536 | 1000000 | fixed while thinking | yes | only without thinking | no | high |
| `qwen/custom` | disabled, **high** | 65536 | 0 | fixed while thinking | yes | only without thinking | no | high |
| `typesafe/jev-latest` | **disabled** | 0 | 64000 | fixed | no | never | no | none |
| `typesafe/jev-1.13.0` | **disabled** | 0 | 64000 | fixed | no | never | no | none |
| `typesafe/custom` | **disabled** | 0 | 0 | fixed | no | never | no | none |
| `xai/grok-4.5` | low, medium, **high** | 500000 | 500000 | adjustable | yes | always | yes | none |
| `xai/grok-4.20-reasoning` | **high** | 1000000 | 1000000 | adjustable | yes | always | yes | none |
| `xai/grok-4.20` | **disabled** | 1000000 | 1000000 | adjustable | yes | always | yes | none |
| `xai/custom` | **disabled**, low, medium, high, extra_high, max | u32::MAX | 0 | adjustable | yes | always | yes | none |

<!-- capability-matrix:end -->
