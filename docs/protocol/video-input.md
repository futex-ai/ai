# Video Input Protocol

## Purpose And Status

This protocol defines the shared video content part for conversation messages
and its provider mappings, mirroring the existing image content part. It is
implemented by the
[video input support plan](../../plans/add-video-input-support.md).

`ai-interface` owns the stable content-part DTO and the routing feature.
Provider crates own their native wire mappings and typed rejections.
Composition roots own routing, retries, storage, and base64 encoding of video
bytes.

## Shared Boundary

`ConversationContentPart` gains a third variant alongside `Text` and `Image`:

```rust
ConversationContentPart::Video {
    mime_type: String,
    data_base64: String,
}
```

- `mime_type` declares the video container format, for example `video/mp4` or
  `video/webm`.
- `data_base64` carries the base64-encoded video bytes.

The variant serializes with the same externally tagged layout as the other
parts:

```json
{ "type": "video", "mime_type": "video/mp4", "data_base64": "..." }
```

Video parts follow the same placement rules as image parts: they are authored
through `ConversationMessage::user_with_parts` on user messages, ordered
freely between text and image parts, and the plain `content` string remains
the text fallback that providers must not send when parts are present.

The shared interface performs no MIME validation and imposes no size limit,
matching image behavior. Providers and upstream APIs enforce their own format
and size rules; oversized or unsupported payloads surface as provider errors.
Base64 inline transport is the entire V1 contract. File/URL references,
provider file-upload APIs, frame sampling, and video generation are out of
scope.

## Routing Feature

`ModelFeature::VideoInput` (config id `video_input`) advertises that a catalog
model accepts video content parts. Callers that attach video parts should
route with `ModelRequirement::Feature(ModelFeature::VideoInput)` so fallback
chains only contain video-capable models.

Catalog models advertising `video_input`:

| Provider | Catalog ids |
| --- | --- |
| Google | `gemini-3.6-flash`, `gemini-3.6-flash-thinking-high`, `gemini-3.5-flash-lite` |
| MiniMax | `MiniMax-M3`, `MiniMax-M3-thinking-disabled` |

`gemini-3.1-flash-image` is an image-generation model and does not accept
video. MiniMax M2.7 models are text/tool models without vision or video.

## Provider Mappings

### Google

Video parts map to the same `inlineData` part shape as images:

```json
{ "inlineData": { "mimeType": "video/mp4", "data": "<base64>" } }
```

Gemini natively documents inline video understanding for common containers
(`video/mp4`, `video/mpeg`, `video/mov`, `video/avi`, `video/x-flv`,
`video/mpg`, `video/webm`, `video/wmv`, `video/3gpp`). The adapter passes the
declared MIME type through unchanged. Inline `generateContent` requests carry
an upstream total-request size limit of roughly 20 MB; larger videos require
the Google File API, which is outside this contract.

### MiniMax

MiniMax-M3 supports video input. Video parts map to the OpenAI-compatible
`video_url` content part using a data URL, exactly parallel to the existing
`image_url` mapping:

```json
{ "type": "video_url", "video_url": { "url": "data:video/mp4;base64,<data>" } }
```

Only the M3 catalog entries advertise `video_input`. M2.7 requests that carry
video parts still serialize the `video_url` part; MiniMax rejects them
upstream because those models are not vision models. Deterministic wire tests
cover the serialization; the implementation environment for this change had no
MiniMax credential, so the billable live assertion is deferred to the
credentialed workflow.

### Rejecting Providers

Anthropic, OpenAI, xAI, Kimi, and Qwen have no documented video-input wire
contract. Their request mappers return a typed model-boundary error before
transport when any message contains a video part:

```
ModelError::Provider { provider, model_id,
    message: "<Provider> accepts text and image content parts only" }
```

DeepSeek keeps its existing stricter rule: every typed content part, including
video, is rejected with "DeepSeek accepts plain text messages only".

Rejection is a `ModelError::Provider` value, so `ai-models-multi` fallback
chains skip past a non-video model to the next candidate, and retry wrappers
do not retry it. No provider silently drops or transforms a video part.

## Identity Hashing

`ai-models-core` includes video parts in the deterministic synthetic
tool-call scope hash with the tag `video`, followed by the MIME type and the
base64 payload, mirroring image hashing. Two requests that differ only in a
video part therefore hash to different scopes.

## Verification

Unit tests are credential-free and cover:

- serde round trips for the `video` content part in `ai-interface`
- the `video_input` feature id and routing metadata
- Google request serialization of ordered text, image, and video parts
- MiniMax request serialization of `video_url` data-URL parts
- typed rejection in Anthropic, OpenAI, xAI, Kimi, and Qwen mappers
- DeepSeek's existing all-parts rejection covering video parts
- identity-hash sensitivity to video parts in `ai-models-core`
- catalog feature assertions for Google and MiniMax video models

Live credentialed coverage for video prompts is not part of this change; the
chat live suite continues to send text-only probes. Extending the live suite
with a small video fixture is future work tracked in the plan.

## Official References

- [Google video understanding guide](https://ai.google.dev/gemini-api/docs/video-understanding)
- [Google generateContent API reference](https://ai.google.dev/api/generate-content)
- [MiniMax model invocation](https://platform.minimax.io/docs/guides/text-generation)
- [MiniMax OpenAI Chat Completions API](https://platform.minimax.io/docs/api-reference/text-chat-openai)
