# Image Generation Protocol

## Purpose And Status

This protocol defines a provider-agnostic, one-image generation and editing
boundary for agent runtimes. It is implemented by the completed
[image generation implementation plan](../../plans/add-image-generation-support.md).

`ai-interface` owns the stable DTO, error, and trait contract.
`ai-models-openai` and `ai-models-google` own provider transport and mapping.
Composition roots own credentials, routing, retries, storage, retention,
pricing, and wire-level base64 encoding.

Credentialed production-API verification is defined by the implemented
[live image API test protocol](live-image-api-tests.md).

## Shared Boundary

`ImageGenerator::generate` accepts an `ImageGenerationRequest` and returns an
`ImageGenerationResponse` asynchronously. Implementations are `Send + Sync` and
are normally held through `DynImageGenerator = Arc<dyn ImageGenerator>`.

The request contains:

- `prompt: String`: required generation/edit instruction. Whitespace-only is
  `EmptyPrompt`.
- `input_images: Vec<ImageGenerationInputImage>`: ordered decoded input bytes
  plus MIME types. Empty selects text-to-image; non-empty selects edit mode.
- `aspect: ImageGenerationAspect`: `Auto`, `Square`, `Landscape`, or
  `Portrait`; `Auto` is the default.
- `quality: ImageGenerationQuality`: `Auto`, `Low`, `Medium`, or `High`; `Auto`
  is the default.

The common accepted edit input types are `image/png`, `image/jpeg`, and
`image/webp`. A provider adapter rejects any other type locally as
`UnsupportedMediaType`. Providers may impose additional upstream byte/count
limits.

Each successful call returns exactly one `GeneratedImage` with decoded bytes
and its provider-reported or deterministically inferred MIME type. The response
also carries provider, configured model id, optional revised prompt, and
`ModelUsage`. Callers request multiple images with multiple trait calls; the
boundary has no count parameter and ignores extra provider images.

## Normalized Controls

Aspect expresses geometry, not exact pixels. Providers map it best-effort:

| Shared aspect | OpenAI `size` | Google aspect ratio |
| --- | --- | --- |
| `Auto` | `auto` | omitted |
| `Square` | `1024x1024` | `1:1` |
| `Landscape` | `1536x1024` | `3:2` |
| `Portrait` | `1024x1536` | `2:3` |

OpenAI maps quality to `auto`, `low`, `medium`, or `high`. Google intentionally
omits quality because its image-size control is not the same semantic contract.
Providers may ignore a normalized control they cannot represent.

## Usage

Only provider-reported usage is normalized; this boundary estimates neither
tokens nor cost. Missing usage becomes `ModelUsage::default()`.

- OpenAI maps `input_tokens`, `output_tokens`, and `total_tokens`. Cached and
  reasoning buckets remain zero.
- Google subtracts `cachedContentTokenCount` from `promptTokenCount` using
  saturating arithmetic, maps `candidatesTokenCount` to ordinary output,
  `thoughtsTokenCount` to reasoning, and uses `totalTokenCount` when present.
  Missing total is reconstructed from the non-overlapping buckets.

## Error Contract

`ImageGenerationError` has these caller-visible classes:

| Variant | Meaning | Retry/fallback behavior |
| --- | --- | --- |
| `EmptyPrompt` | local blank prompt | fix request; do not retry |
| `UnsupportedMediaType` | local unsupported input MIME | fix request; do not retry |
| `ContentPolicy` | provider safety, blocklist, moderation, recitation, or prohibited-content refusal | expose as tool failure so an agent may adapt |
| `NoImage` | successful provider response has no final image | terminal provider result |
| `RateLimited` | HTTP 429 | retry/fail over |
| `TransientProvider` | timeout, transport/auth, HTTP 408/409/425, or 5xx | retry/fail over |
| `Provider` | other provider rejection or malformed provider semantics | terminal/fail according to caller policy |
| `Internal` | local serialization, decoding, or invariant failure | internal failure |

All provider variants retain provider, configured model id, and the available
provider message. `Internal` uses the shared tracked `InternalError` carrier so
the boundary preserves its definition and failure call sites. Error display
strings use the `[ai_interface/image_generator]` prefix. Invalid provider
base64 is `Internal`; a valid response without non-empty final image bytes is
`NoImage`.

## OpenAI Mapping

The current catalog model is `gpt-image-2` (`GPT_IMAGE_2`). It advertises
`ImageGeneration` and `Vision`; the model catalog has no provider-advertised
context window for this specialized endpoint, so its context value is zero.

`OpenAiImageGenerator` uses an injected `DynJsonHttpClient`, explicit API key or
auth hook, a two-minute default timeout, and overridable endpoints for tests:

- No input images: JSON `POST https://api.openai.com/v1/images/generations`.
- Input images present: multipart `POST https://api.openai.com/v1/images/edits`
  with repeated `image[]` parts.

Both send model, prompt, mapped size, mapped quality, and `n = 1`. The adapter
does not request output format, compression, transparency, masks, moderation
level, streaming, or partial images. GPT Image returns base64 in
`data[0].b64_json`; `output_format` maps `png`, `jpeg`, or `webp` to a MIME type,
with byte-signature inference when absent. An optional item `revised_prompt` is
preserved when supplied. Decoded zero-byte payloads are `NoImage`.

HTTP error bodies are decoded enough to inspect `error.code` and `error.type`.
The normalized codes `content_policy_violation`, `moderation_blocked`,
`safety_violation`, and `image_generation_safety_violation` are
`ContentPolicy`. Status 429 wins as `RateLimited`; transient statuses win next;
unrecognized failures are `Provider`.

## Google Mapping

The current catalog model is `gemini-3.1-flash-image`
(`GEMINI_3_1_FLASH_IMAGE`), a stable GA model with a 131,072-token input limit.
It advertises `ImageGeneration` and `Vision`.

`GoogleImageGenerator` uses an injected `DynJsonHttpClient`, explicit API key or
auth hook, and an overridable endpoint. It posts JSON to
`https://generativelanguage.googleapis.com/v1/models/{model}:generateContent`.
The single user content contains the prompt followed by ordered `inline_data`
parts. `generationConfig.responseModalities` is `["IMAGE"]`; a non-auto aspect
is sent as `generationConfig.responseFormat.image.aspectRatio`.

The adapter scans candidates and parts in provider order, skips parts with
`thought: true`, and returns the first final `inlineData` image. Interim thought
images must never become the user-visible result. Quality is intentionally
omitted. An absent or zero-byte final image is `NoImage` unless a policy block
is present.

Prompt `blockReason` values `SAFETY`, `BLOCKLIST`, `PROHIBITED_CONTENT`, and
`IMAGE_SAFETY` are `ContentPolicy`. Candidate finish reasons `SAFETY`,
`RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT`, `SPII`, `IMAGE_SAFETY`,
`IMAGE_PROHIBITED_CONTENT`, `IMAGE_RECITATION`, and `ESCALATION` are also
`ContentPolicy`. `NO_IMAGE` is `NoImage`; other image/provider finish failures
are `Provider`. `finishMessage` is retained when available.

## Verification

Unit tests are credential-free and cover DTO serde/defaults, the deterministic
mock, routing feature, request mapping, response/usage parsing, thought-image
filtering, non-empty image enforcement, tracked internal-error metadata, every
error class, OpenAI transport/auth behavior, and catalog metadata. The
centralized `xtask/tests/live_images/mod.rs` suite owns ignored, catalog-driven
Google and OpenAI production-API checks. It validates provider/model identity,
non-empty bytes, supported MIME types, and matching PNG/JPEG/WebP signatures.
Ordinary workspace checks compile the ignored cases and run their registry,
runner, retry, validation, and workflow guards without credentials or network
access.

The full workspace must pass formatting, Clippy, tests, file-length lint,
credential-free smoke tests, and `cargo xtask check` before commit and push.

## Official References

- [OpenAI image generation guide](https://developers.openai.com/api/docs/guides/image-generation)
- [OpenAI GPT Image 2 model](https://developers.openai.com/api/docs/models/gpt-image-2)
- [Google image generation guide](https://ai.google.dev/gemini-api/docs/image-generation)
- [Google generateContent image generation compatibility guide](https://ai.google.dev/gemini-api/docs/generate-content/image-generation)
- [Google Gemini 3.1 Flash Image model](https://ai.google.dev/gemini-api/docs/models/gemini-3.1-flash-image)
- [Google generateContent API reference](https://ai.google.dev/api/generate-content)
