# Model Policies Protocol

Status: Approved, not implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

When implemented, this protocol replaces the retry and fallback rules of
[model-completion-streaming.md](model-completion-streaming.md) and
[model-completion-events.md](model-completion-events.md), and the wrapper
contract of [provider-call-controls.md](provider-call-controls.md).

This protocol defines the error core, the scope of each error kind, and the
retry, fallback, concurrency, and pricing layers. The
[model stream protocol](model-stream.md) defines how each layer keeps the
event stream.

## Error core

Every error of every modality carries one error core: model calls, image
generation, video generation, audio transcription, and judgment.

| Field | Meaning |
| --- | --- |
| Kind | The error kind |
| Provider | The provider id of the adapter |
| Model | The provider model id of the call |
| Message | The text of the provider or of the adapter |
| Retry-after | The wait that the provider asked for, when it sent one |
| Provider request id | The id of the failed request, when the provider sent one |

- A local error carries the provider and the model of the adapter that
  raised it.
- The retry-after comes from the `Retry-After` header of the failed
  response, in seconds or as an HTTP date.
- The provider request id comes from the response header in which the
  provider returns its request id.
- The HTTP component gives these headers for buffered responses, for
  streams, and for status errors.
- A message never holds a credential or an authorization header.

## Error kinds

| Kind | Meaning | Another model can succeed | Retried |
| --- | --- | --- | --- |
| Invalid request | The request breaks the contract, or the provider rejects it as invalid: HTTP 400, 413, 422, or another 4xx status with no more specific kind | no | no |
| Rejected key | The provider rejects the key or its account: HTTP 401, 402, or 403 | yes | no |
| Unknown model | The provider does not know the model id: HTTP 404 | yes | no |
| Unsupported input | The model does not take a part or a media type of the request | yes | no |
| Invalid output | The output of the provider breaks the response contract | yes | no |
| Content policy | The provider refuses the request or the output for policy reasons | no | no |
| Rate limited | HTTP 429, or a rate-limit code of the provider | yes | yes |
| Transient | The provider is overloaded or failed for a short time, before progress: HTTP 408, 409, 425, or 5xx, or a transport failure | yes | yes |
| Interrupted | A stream failed after progress | yes | no |
| Context limit exceeded | The input is larger than the context window of the model | yes | no |
| Unsupported control | The model cannot honour an explicit control. The kind names the control | yes | no |
| Timed out | A generation job did not finish before its total timeout | yes | no |
| Internal | A local failure: serialization, a broken invariant, or a missing transport capability | no | no |

- Each kind answers whether another model can succeed, as the table says.
- A context-limit code in an HTTP 400 body gives the context limit exceeded
  kind.
- The provider docs map the error codes of their providers to kinds.
- Only image and video adapters give the content policy kind. Only video
  adapters give the timed out kind.
- The typed errors of the other modalities map to kinds:

| Modality error | Kind |
| --- | --- |
| Empty prompt, empty audio, empty state, no questions, invalid question, unsupported content | Invalid request |
| Unsupported media type, audio too large | Unsupported input |
| No image, no video, empty transcript, invalid answer | Invalid output |

- Every other provider failure of a modality takes its kind from the kinds
  table, for example from its HTTP status.

## Retry

- The retry layer wraps one model.
- It retries the rate limited and transient kinds only.
- It never retries the interrupted kind, because the provider can bill the
  partial generation twice.
- It makes at most three attempts by default: the first call and two
  retries.
- The base delays of the two retries are 100 ms and 250 ms.
- Each wait is a random duration from half of its base delay to its full
  base delay.
- When the error carries a retry-after, the wait is the larger of the
  retry-after and the random duration.
- When the retry-after is longer than the maximum wait, the layer does not
  retry and returns the error. The default maximum wait is 30 s.
- The caller can set the number of attempts, the base delays, and the
  maximum wait.
- Each attempt gets the full total timeout of the request. A wait does not
  count against it.
- Decision: retry backs off with jitter, honours retry-after, and covers
  rate limits. The rejected alternative retried only transient failures on a
  fixed schedule. It retried at the same moment as other callers and ignored
  the wait that the provider asked for.

## Fallback

- A fallback layer holds an ordered list of models and a fallback policy.
- A fallback layer has no default policy. Each fallback states its policy.
- The preset "any error" moves on after every error kind.
- The preset "only when another model can succeed" moves on only after a
  kind whose table entry says yes.
- A custom policy is a rule of the caller that decides from the error of
  the failed attempt.
- After a failed attempt, the layer asks its policy. When the policy moves
  on and a model remains, the layer sends the unchanged request to the next
  model.
- When the policy stops, or when no model remains, the layer returns the
  error of the last attempt. That error lists every earlier attempt.
- A successful response lists every failed attempt before it.
- An attempt record holds the provider id, the model id, the error kind,
  the message, and whether the attempt emitted events.
- A fallback layer with no models fails every call with the internal kind.
- Both presets move on after the interrupted kind, so that both attempts can
  bill. A caller that cannot accept that uses a custom policy that stops on
  the interrupted kind.
- Decision: a fallback needs an explicit policy, with the presets "any
  error" and "only when another model can succeed". Benchmarks and products
  need different rules. An undivided provider error stops a chain on
  failures that the next model can take. The rejected alternatives were
  these:
  - "Any error" as the default.
  - "Only when unavailable" as the default.
  - A required policy with presets on classes that keep one undivided
    provider error.
  - No fallback in the library.

## Concurrency

- The concurrency layer allows at most a set number of calls of its model
  at the same time. A limit of zero sets no limit.
- A call holds its permit as the model stream protocol says.
- Waiting calls get their permits in arrival order.

## Pricing

- The pricing layer applies a deployment price snapshot, `ModelPricing`, to
  the usage of its model.
- A snapshot has an optional price for input, output, cached input, and
  reasoning tokens, in micro-USD for one million tokens. It also has an
  optional rate version and a free-when-unpriced flag.
- The layer writes one cost line for each token kind with a quantity above
  zero, in the order input, output, cached input, and reasoning.
- The cost of a line is the quantity times the price, divided by one
  million, rounded half up, in micro-USD.
- A line is free when its price is zero, or when it has no price and the
  free-when-unpriced flag is set. A free line costs zero.
- A line is measured when it has a price above zero, and unknown when it has
  no price and the flag is not set. An unknown line has no cost.
- Each line carries the rate version of the snapshot.
- The estimated cost is the sum of the known line costs.
- An adapter reports token quantities only. Its estimated cost is zero and
  its cost lines are empty until a pricing layer prices them.
- Prices never come from model specs, because deployment prices change
  without a release.

## Composition

- Each policy layer is a `Model` and wraps any `Model`.
- Each policy layer passes the request on unchanged.
- A policy layer decides from error kinds only. It never decides from a
  provider id or from the text of an error.
- A host can stack the layers in any order.
- A pricing layer prices with the prices of the model that it wraps. A host
  that prices a fallback chain wraps each model of the chain before the
  fallback layer.
