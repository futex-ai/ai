# ai-models-qwen

`ai-models-qwen` connects the shared AI model interface to QwenCloud's
OpenAI-compatible Chat Completions API. Depend on it when a composition root
needs Qwen 3.7 Max, Plus, or Flash with explicit thinking behavior.

## Responsibilities

- Publish the supported Qwen 3.7 catalog and routing metadata.
- Map shared conversations, tools, images, and structured-output requests to
  QwenCloud.
- Preserve Qwen reasoning and raw tool calls for exact multi-turn replay.
- Normalize responses, usage, and provider errors into `ai-interface` types.
- Accumulate completion SSE internally while preserving the buffered model
  interface.

## What This Crate Does

The adapter targets QwenCloud's international pay-as-you-go endpoint. It
supports high-thinking and thinking-disabled variants, parallel function
calling, Plus/Flash image input, and locally validated JSON objects. Qwen 3.7
Max is intentionally text-only, and every model rejects shared video content
parts with a typed provider error before transport. Replay emits null assistant content only for
tool-call turns, and invalid provider tool-call identities are rejected before
they reach the shared runtime.

Portable sampling maps only when thinking is disabled; enabled thinking keeps
provider sampling defaults. Output limits, stops, and supported tool choices
map natively, forced thinking-mode choices are typed as unsupported, blank
system prompts are omitted, and per-call timeouts reach the transport. The
typed `RequiredOrAuto` policy forces required use without thinking and retains
tools with automatic selection in thinking mode.

Unsupported normalized levels downgrade without exceeding the request: low
and medium resolve to disabled, while extra-high and max resolve to high.
Responses record the effective level.

Every completion requests streamed usage and accumulates content, private
reasoning, indexed tool calls, finish state, and the final usage chunk through
the existing response mapper. Streams default to a 3,600-second overall
deadline and a 120-second idle timeout; a caller timeout replaces the overall
deadline. Failures after any event become `ModelError::Interrupted`.

Public incremental streaming, Qwen Coding Plan endpoints, preview model
snapshots, built-in tools, and ambient credential lookup are outside this
crate's boundary.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_qwen::QwenModel;
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
let model = QwenModel::new(client, "qwen-api-key");
```

The default is the high-thinking `qwen3.7-plus` catalog model. Use
`QwenModel::with_catalog_auth` to construct another exported catalog variant.

## Development

From the workspace root:

```sh
cargo test -p ai-models-qwen
cargo clippy -p ai-models-qwen --all-targets --all-features -- -D warnings
cargo xtask check
```

### Key Code

- `src/catalog.rs` — known Qwen models and routing metadata.
- `src/qwen/client.rs` — validated construction, dispatch, and HTTP errors.
- `src/qwen/request.rs` — request, replay, multimodal, and schema mapping.
- `src/qwen/response.rs` — response, tool-call, replay, and usage normalization.
- `src/qwen/stream.rs` — SSE accumulation and progress-aware failures.

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/qwen-model-provider.md`](../../docs/protocol/qwen-model-provider.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
