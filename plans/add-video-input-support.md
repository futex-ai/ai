# Add Video Input Support

Add a shared video content part to conversation messages so video-capable
providers receive inline video the same way they receive inline images, as
defined by the [video input protocol](../docs/protocol/video-input.md).

Google Gemini chat models map video to native `inlineData` parts and
MiniMax-M3 maps video to OpenAI-compatible `video_url` data URLs. Anthropic,
OpenAI, xAI, Kimi, and Qwen return typed provider errors before transport;
DeepSeek keeps rejecting every typed content part. A new
`ModelFeature::VideoInput` routing feature advertises the capable catalog
models.

## Milestone 1: Protocol Docs

Define the shared contract before implementation.

- [x] Write `docs/protocol/video-input.md` covering the DTO, serde layout,
      provider mappings, rejection semantics, routing feature, hashing, and
      verification contract
- [x] Update `docs/protocol/minimax-model-provider.md` request mapping and
      catalog features for M3 video input
- [x] Update `docs/protocol/kimi-model-provider.md` and
      `docs/protocol/qwen-model-provider.md` to state typed video rejection
- [x] Link the new protocol doc from the workspace `README.md`

## Milestone 2: Shared Part, Provider Mappings, And Tests

One atomic code change so the workspace never stops compiling: the new
variant, every provider match arm, catalogs, and full deterministic tests.

- [x] Add `ConversationContentPart::Video { mime_type, data_base64 }` in
      `ai-interface` with serde round-trip tests
- [x] Add `ModelFeature::VideoInput` with config id `video_input` and router
      tests
- [x] Hash video parts in `ai-models-core` synthetic tool-call identity with
      tests
- [x] Google: map video parts to `inlineData`, advertise `video_input` on
      Gemini chat models, extend multimodal and catalog tests
- [x] MiniMax: add `video_url` wire part, map video parts to data URLs,
      advertise `video_input` on M3 entries, extend multimodal and catalog
      tests
- [x] Anthropic, OpenAI, xAI, Kimi, Qwen: make content-part mapping fallible
      and return typed provider errors for video parts, with rejection tests
- [x] DeepSeek: cover video parts under the existing all-parts rejection test

## Milestone 3: Docs, Verification, And Review

- [x] Update workspace and crate READMEs (root, `ai-interface`,
      `ai-models-google`, `ai-models-minimax`, and rejecting-provider READMEs
      where they describe multimodal input)
- [x] Run `cargo fmt --all -- --check`, `cargo clippy`, and the full test
      suite
- [x] Run `cargo xtask check`
- [x] Commit the work with a Conventional Commit message and push the branch
- [x] Run `cargo xtask review` against `origin/main` and report findings
      without auto-fixing

## Future Work

- Live credentialed video probes for Google and MiniMax with a small video
  fixture, plus a `docs/protocol/live-model-api-tests.md` update
- Video file/URL references and provider file-upload APIs for payloads larger
  than inline limits
- A `VideoGenerator` boundary (OpenAI Sora, Google Veo) mirroring
  `ImageGenerator` if video generation is needed
