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

## What This Crate Does

The adapter targets QwenCloud's international pay-as-you-go endpoint. It
supports high-thinking and thinking-disabled variants, parallel function
calling, Plus/Flash image input, and locally validated JSON objects. Qwen 3.7
Max is intentionally text-only. Replay emits null assistant content only for
tool-call turns, and invalid provider tool-call identities are rejected before
they reach the shared runtime.

Streaming, Qwen Coding Plan endpoints, preview model snapshots, built-in tools,
and ambient credential lookup are outside this crate's boundary.

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

### Related Docs

- [`../../docs/protocol/qwen-model-provider.md`](../../docs/protocol/qwen-model-provider.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
