# Video Input Protocol

## Purpose And Status

This protocol defines the shared video content part for conversation messages
and its provider mappings, mirroring the existing image content part. It is
implemented by the
[video input support plan](../../plans/add-video-input-support.md).

`ai-interface` owns the shared video part and the routing feature.
Provider crates own their native wire mappings and typed rejections.
Composition roots own routing, retries, storage, and base64 encoding of video
bytes.

## Shared Boundary

The [conversation parts protocol](conversation-parts.md) defines the video
part beside the text and image parts:

```rust
ConversationPart::Video {
    mime_type: String,
    data_base64: String,
    provider_state: Option<ProviderState>,
}
```

- `mime_type` declares the video container format, for example `video/mp4` or
  `video/webm`.
- `data_base64` carries the base64-encoded video bytes.
- `ConversationPart::video(mime_type, data_base64)` builds a video part
  without provider state.

The part serializes with the same `type` tag as the other parts:

```json
{ "type": "video", "mime_type": "video/mp4", "data_base64": "..." }
```

Video parts follow the same placement rules as image parts. They belong to
user messages. `ConversationMessage::user_with_parts` builds a user message
from one ordered list of parts, and text, image, and video parts can come in
any order. A user message has no separate text fallback: its text parts are
its text.

The shared interface performs no MIME validation and imposes no size limit,
matching image behavior. Providers and upstream APIs enforce their own format
and size rules; oversized or unsupported payloads surface as provider errors.
Base64 inline transport is the entire V1 contract. File/URL references,
provider file-upload APIs, frame sampling, and video generation are out of
scope.

## Routing Feature

`ModelFeature::VideoInput` (config id `video_input`) advertises that a model
spec accepts video content parts. Callers that attach video parts should
route with `ModelRequirement::Feature(ModelFeature::VideoInput)` so fallback
chains only contain video-capable models.

Model specs advertising `video_input`:

| Provider | Text ids |
| --- | --- |
| Google | `google/gemini-3.6-flash`, `google/gemini-3.5-flash-lite` |
| MiniMax | `minimax/MiniMax-M3` |

`google/gemini-3.1-flash-image` is an image-generation model and does not
accept video. MiniMax M2.7 models are text/tool models without vision or
video.

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

Only the M3 spec advertises `video_input`. M2.7 requests that carry
video parts still serialize the `video_url` part; MiniMax rejects them
upstream because those models are not vision models. Deterministic wire tests
cover the serialization; the implementation environment for this change had no
MiniMax credential, so the billable live assertion is deferred to the
credentialed workflow.

### Rejecting Providers

Anthropic, OpenAI, xAI, Kimi, and Qwen have no documented video-input wire
contract. Their request mappers fail before transport when any message
contains a video part. The error has the unsupported input kind, the
provider and model id of the adapter, and the message
"<Provider> accepts text and image content parts only".

DeepSeek keeps its existing stricter rule: every image or video part is
rejected with "DeepSeek accepts plain text messages only". DeepSeek accepts
text parts.

Rejection has the unsupported input kind, so both fallback presets move past
a non-video model to the next candidate, and the retry layer does not retry
it. No provider silently drops or transforms a video part.

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
- DeepSeek's existing image and video rejection covering video parts
- identity-hash sensitivity to video parts in `ai-models-core`
- spec feature assertions for Google and MiniMax video models

Live credentialed coverage for video prompts is not part of this change; the
chat live suite continues to send text-only probes. Extending the live suite
with a small video fixture is future work tracked in the plan.

## Official References

- [Google video understanding guide](https://ai.google.dev/gemini-api/docs/video-understanding)
- [Google generateContent API reference](https://ai.google.dev/api/generate-content)
- [MiniMax model invocation](https://platform.minimax.io/docs/guides/text-generation)
- [MiniMax OpenAI Chat Completions API](https://platform.minimax.io/docs/api-reference/text-chat-openai)
