# Video Generation Protocol

## Status

Implemented by the
[video generation implementation plan](../../plans/add-video-generation-support.md).

## Boundary

`VideoGenerator::generate` accepts one `VideoGenerationRequest` and resolves
only after one complete video has been downloaded. Implementations are
`Send + Sync` and normally held through
`DynVideoGenerator = Arc<dyn VideoGenerator>`.

The boundary deliberately hides provider job submission, polling, and asset
download. It is appropriate for workers and other long-running callers. HTTP
request lifetimes, queue persistence, cancellation, webhooks, and progress UI
belong to the consumer.

## Request And Response

`VideoGenerationRequest` contains:

- `prompt: String`: required non-blank text instruction.
- `input_image: Option<VideoGenerationInputImage>`: optional decoded JPEG,
  PNG, or WebP first frame. It selects image-to-video generation.
- `aspect: VideoGenerationAspect`: `Landscape` (default) or `Portrait`.
- `duration: VideoGenerationDuration`: `Seconds4` (default) or `Seconds8`.
- `resolution: VideoGenerationResolution`: currently `P720` (default).

The controls are the portable intersection of the Sora 2 and Veo 3.1 APIs
that the boundary first targeted. Provider-only durations and resolutions are
excluded instead of being silently changed by one provider.

`VideoGenerationResponse` contains provider and configured provider-model
identity, exactly one non-empty `GeneratedVideo`, and `ModelUsage`. The video
contains decoded MP4 bytes, `video/mp4`, duration seconds, width, and height.
Provider APIs currently expose no normalized token usage for these calls, so
usage is `ModelUsage::default()`. Storage and retention belong to callers.

## Error Contract

`VideoGenerationError` has these caller-visible classes:

| Variant | Meaning | Retry/fallback behavior |
| --- | --- | --- |
| `EmptyPrompt` | local blank prompt | fix request; do not retry |
| `UnsupportedMediaType` | local unsupported input-image MIME | fix request; do not retry |
| `ContentPolicy` | provider safety or moderation refusal | expose so the caller may adapt |
| `NoVideo` | completed response or download has no video bytes | terminal provider result |
| `TimedOut` | the configured total generation deadline elapsed | retry/fail over according to caller policy |
| `RateLimited` | HTTP 429 | retry/fail over |
| `TransientProvider` | transport failure or HTTP 408/409/425/5xx | retry/fail over |
| `Provider` | terminal provider job or malformed provider semantics | terminal/fail according to caller policy |
| `Internal` | serialization or local invariant failure | internal failure |

All provider variants retain provider, configured provider model id, and the
available provider message. `Internal` uses tracked `InternalError` metadata.
Display strings use the `[ai_interface/video_generator]` prefix.

## Polling Contract

Production adapters use an injected `DynPollingRuntime`, a ten-second poll
interval, and a ten-minute total generation deadline by default. The runtime
combines a monotonic clock with async sleeping so tests can deterministically
advance time. The deadline covers job submission, polling sleeps, status
requests, and the final download. A provider job that is still queued or
running at the deadline returns `TimedOut`.

## Providers

The Google adapter is the only video adapter. No provider catalog lists a
video model. OpenAI has no video adapter, because OpenAI no longer serves its
video API.

## Google Mapping

The Google catalog lists no video model. Google no longer serves
`veo-3.1-generate-preview`, and no successor model has passed a live check.
Callers pass the provider model id to `GoogleVideoGenerator`.

`GoogleVideoGenerator` uses injected JSON HTTP and auth collaborators:

1. `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:predictLongRunning`
   submits `instances` and `parameters`.
2. `GET https://generativelanguage.googleapis.com/v1beta/{operation}` is
   polled until `done`.
3. The first generated sample URI is downloaded with the same API-key auth.

The request contains exactly one instance with `prompt` and optional
`image.inlineData`. Parameters contain `aspectRatio` (`16:9` or `9:16`),
`durationSeconds` (`4` or `8`), `resolution: "720p"`, and
`sampleCount: 1`. Completed operations read
`response.generateVideoResponse.generatedSamples[0].video.uri`. Operation or
sample safety metadata is `ContentPolicy`; a completed response without a URI
or a zero-byte download is `NoVideo`.

## Verification

Credential-free tests cover DTO serde/defaults, the deterministic mock,
routing capability, request mapping, first-frame validation, provider status
and safety classification, job polling, timeouts, authenticated binary
downloads, MP4 non-empty enforcement, catalog metadata, and production-adapter
construction. No live test covers video generation, because no catalog lists
a video model.

## Official References

- [Google Veo video generation guide](https://ai.google.dev/gemini-api/docs/veo)
