# Provider Registry Protocol

Status: Implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

This protocol governs how hosts build adapters through the registry. The
per-provider constructors `new`, `with_auth`, and `with_catalog_auth` remain
until the user answers the
[open constructor default-effort decision](../../plans/follow-ups/constructor-default-effort.md).
The provider protocol docs and
[provider call controls](provider-call-controls.md) describe those
constructors.

This protocol defines the provider config, base URLs, auth, extra headers,
timeouts, the adapters that the registry builds from a model spec, and the
converter that the registry exposes.
[Model capabilities](model-capabilities.md) defines model specs and the
combined catalog.

## Provider config

`ProviderConfig` holds the settings of one provider:

| Field | Meaning | When not set |
| --- | --- | --- |
| Provider | The `ProviderId` of the provider | Always set |
| Base URL | The scheme, the host, and an optional path prefix of every request | The default base URL of the provider |
| Auth | An API key or an auth hook | Always set |
| Extra headers | Headers that every request of the provider carries | No extra headers |
| Timeouts | The idle timeout, the total timeout, and the poll interval | The adapter defaults |

## Base URL

- Each adapter builds every request URL from the base URL of its config and
  a fixed path.
- A trailing slash on the base URL changes no URL.
- An adapter sends the credentials of its config only to the origin of its
  base URL. An origin is the scheme, host, and port.
- The Google video adapter downloads a sample URI that the provider returns
  only when the URI uses HTTPS on the host of the base URL. Otherwise the
  call fails with the invalid output kind.

| Provider | Default base URL | Paths that the adapters add |
| --- | --- | --- |
| `anthropic` | `https://api.anthropic.com` | `/v1/messages` |
| `openai` | `https://api.openai.com` | `/v1/responses`, `/v1/images/generations`, `/v1/images/edits`, `/v1/audio/transcriptions` |
| `google` | `https://generativelanguage.googleapis.com` | `/v1beta/models/{model}:streamGenerateContent?alt=sse`, `/v1/models/{model}:generateContent`, `/v1beta/models/{model}:predictLongRunning`, `/v1beta/{operation}` |
| `deepseek` | `https://api.deepseek.com` | `/chat/completions` |
| `kimi` | `https://api.moonshot.ai` | `/v1/chat/completions` |
| `qwen` | `https://dashscope-intl.aliyuncs.com/compatible-mode` | `/v1/chat/completions` |
| `minimax` | `https://api.minimax.io` | `/v1/chat/completions` |
| `xai` | `https://api.x.ai` | `/v1/chat/completions`, `/v1/chat/deferred-completion/{request id}` |
| `typesafe` | `https://api.typesafe.ai` | `/v1/systemone` |

Decision: every adapter honours the base URL of its config. The rejected
alternative let some adapters keep a fixed endpoint, so that a proxy or a
regional endpoint covered only part of a provider.

## Auth

- Auth is an API key or an auth hook (`DynJsonHttpAuth`).
- With an API key, each adapter sends the key in the scheme of its
  provider:

| Provider | Scheme |
| --- | --- |
| `anthropic` | `x-api-key` header |
| `google` | `x-goog-api-key` header |
| Every other provider | `Authorization: Bearer <key>` |

- With an auth hook, each adapter applies the hook to every request instead.
- A failure of the auth hook has the transient kind.
- The Anthropic adapter also sends `anthropic-version: 2023-06-01`.
- An API key never appears in an error, a log, `Debug` output, or a
  snapshot.

## Extra headers

- Every extra header counts as a credential of its config.
- Each adapter adds every extra header only to requests on the origin of
  the base URL.
- Header names compare without case.
- When an extra header has the name of a header that the adapter or the
  auth sets, the request carries the value of the adapter or the auth.

## Timeouts

| Adapter | Idle timeout | Total timeout | Poll interval |
| --- | ---: | ---: | ---: |
| Every streaming model adapter | 120 s | 3,600 s | None |
| xAI deferred lifecycle | None | 60 s | 5 s |
| OpenAI and Google image adapters | None | 120 s | None |
| OpenAI transcription adapter | None | 60 s | None |
| TypeSafe judgment adapter | None | 60 s | None |
| Google video adapter | None | 600 s | 10 s |

- A timeout that the config sets replaces that default for every adapter of
  the provider that has the timeout.
- The total timeout control of a model request replaces the total timeout
  for that call. It does not change the idle timeout.
- The injected HTTP client owns the connect timeout.

## Building adapters

- `Registry` holds one injected HTTP client and the provider configs.
- The registry holds at most one config for each provider id. A new config
  for a provider replaces the earlier config.
- `Registry::model(&spec)` builds a `Model`. Sibling methods build an image
  generator, a video generator, an audio transcriber, and a judgment model.
- Each built adapter uses the HTTP client of the registry and the base URL,
  auth, extra headers, and timeouts of its provider config.
- A built model applies the capability check with the spec that built it.
- The registry builds adapters without policy layers. The host composes
  the layers of [model policies](model-policies.md).
- Building sends no request. It reads no environment variable and no secret
  store.

| Provider | Model | Image | Video | Transcription | Judgment |
| --- | --- | --- | --- | --- | --- |
| `anthropic` | yes | no | no | no | no |
| `openai` | yes | yes | no | yes | no |
| `google` | yes | yes | yes | no | no |
| `deepseek`, `kimi`, `qwen`, `minimax`, `xai` | yes | no | no | no | no |
| `typesafe` | no | no | no | no | yes |

- A build fails with a typed error when the registry has no config for the
  provider of the spec.
- A build fails with a typed error when the registry does not include the
  provider of the spec.
- A build fails with a typed error when the provider has no adapter of the
  requested modality.
- No catalog lists a video model or a transcription model. A host builds
  these adapters from custom specs.
- The registry has one Cargo feature for each provider. It includes exactly
  the providers whose features are on.
- Decision: one registry builds every adapter from a spec and a provider
  config. It is the preferred single factory for hosts. The rejected
  alternative made per-provider constructors such as `new`, `with_auth`, and
  `with_catalog_auth` the normal factory. Each host and each test then
  repeated a match over providers.

## Converter

- The registry exposes the one converter that hosts call to read stored
  messages of format version 1.
- The converter composes the version 1 conversions of the providers that
  the registry includes.
- [Conversation parts](conversation-parts.md) defines the converter, the
  conversion of each provider, and the decision that places the
  composition in the registry.
