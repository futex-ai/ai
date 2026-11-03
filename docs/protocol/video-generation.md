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

The controls define the portable video generation boundary.
Provider-only durations and resolutions are
excluded instead of being silently changed by one provider.

`VideoGenerationResponse` contains provider and configured provider-model
identity, exactly one non-empty `GeneratedVideo`, and `ModelUsage`. The video
contains decoded MP4 bytes, `video/mp4`, duration seconds, width, and height.
Provider APIs currently expose no normalized token usage for these calls, so
usage is `ModelUsage::default()`. Storage and retention belong to callers.

## Error Contract

Every `VideoGenerationError` carries one error core of the
[model policies protocol](model-policies.md#error-core). The core kind, not
the variant, gives the failure category. The kind decides retry and
fallback, as the [error kinds table](model-policies.md#error-kinds) says.

| Cause | Kind |
| --- | --- |
| `EmptyPrompt`: local blank prompt | invalid request |
| `UnsupportedMediaType`: local unsupported input-image MIME | unsupported input |
| `NoVideo`: completed response or download has no video bytes | invalid output |
| provider safety or moderation refusal | content policy |
| the configured total generation deadline elapsed | timed out |
| HTTP 429 | rate limited |
| transport failure or HTTP 408/409/425/5xx | transient |
| HTTP 401, 402, or 403 | rejected key |
| HTTP 404 | unknown model |
| HTTP 400 with a recognized context-limit code | context limit exceeded |
| other provider rejection | invalid request |
| terminal provider job without a policy or status code, or malformed provider semantics | invalid output |
| serialization or local invariant failure | internal |

The typed variants keep their detail fields. `Failure` holds every other
classified failure. `Internal` holds a failure with its tracked
`InternalError` source. Every core holds the provider, the configured
provider model id, and the available provider message. A content-policy
failure is exposed so the caller may adapt. A recognized context-limit code
in an HTTP 400 body wins over a policy code in the same body. Display
strings use the `[ai_interface/video_generator]` prefix.

When a transport failure ends at or after the total generation deadline, it
has the timed out kind. A provider status received before that deadline keeps
its status kind.

## Polling Contract

Production adapters use an injected `DynPollingRuntime`, a ten-second poll
interval, and a ten-minute total generation deadline by default. The runtime
combines a monotonic clock with async sleeping so tests can deterministically
advance time. The deadline covers job submission, polling sleeps, status
requests, and the final download. A provider job that is still queued or
running at the deadline fails with the timed out kind.

## Providers

The Google adapter is the only video adapter. No provider catalog lists a
video model. OpenAI has no video adapter, because OpenAI no longer serves its
video API.

## Google Mapping

The Google catalog lists no video model. Google no longer serves the retired
preview model, and no successor model has passed a live check.
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
sample safety metadata has the content policy kind; a completed response
without a URI or a zero-byte download is `NoVideo`. An operation that is not
done stays pending, also when it holds an error. A done operation with an
error takes the kind of its canonical RPC code or numeric HTTP status from
the shared status classification. An unknown code has the invalid output
kind.

## Verification

Credential-free tests cover DTO serde/defaults, the deterministic mock,
routing capability, request mapping, first-frame validation, provider status
and safety classification, job polling, timeouts, authenticated binary
downloads, MP4 non-empty enforcement, catalog metadata, and production-adapter
construction. No live test covers video generation, because no catalog lists
a video model.

## Official References

- [Google Veo video generation guide](https://ai.google.dev/gemini-api/docs/veo)
