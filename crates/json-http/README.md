# json-http

`json-http` is a small typed HTTP client crate for workspace services and
provider clients. Depend on it when you need a reusable builder-style API over
JSON requests, multipart uploads, authenticated binary downloads, or decoded
SSE streams without embedding reqwest boilerplate in each caller.

## Responsibilities

- Provide a trait-backed HTTP client boundary for JSON, multipart, binary, and
  Server-Sent Events response requests
- Expose a builder API for `get`, `post`, `put`, `delete`, and `patch`
- Support reusable auth hooks that apply request headers before dispatch
- Decode SSE incrementally while enforcing connect, idle, and overall timeouts

## What This Crate Does

`json-http` wraps HTTP calls behind `JsonHttpClient`. Requests are built with
`JsonHttpRequestBuilder`, request bodies are serialized from typed Rust structs
or attached as multipart byte fields, and responses can be read as raw bytes,
`serde_json::Value`, or deserialized into typed response DTOs.

The crate ships with:

- `ReqwestJsonHttpClient` for real transport
- `TransportBackedJsonHttpClient` for tests or alternate transports
- `StaticHeaderAuth` for simple header-based auth injection
- feature-gated `JsonHttpTransportMock` and `JsonHttpAuthMock` boundaries for
  credential-free provider tests
- feature-gated `JsonHttpSseStreamMock`, a pure incremental SSE decoder, and a
  pull-based decoded event stream
- `JsonHttpMultipartField` for small multipart upload calls that still use the
  same auth hooks and response handling
- 600-second buffered request timeouts, a 10-second reqwest connect timeout,
  and optional SSE idle timeouts within one overall request deadline
- `send_bytes()` for authenticated downloads through the same transport and
  auth hooks as JSON calls

`JsonHttpTransport::execute_sse` defaults to the typed `SseUnsupported` error.
Existing alternate transports therefore continue to compile, but must
implement streaming before callers use `send_sse()`. Successful streams require
`text/event-stream`; non-success responses retain at most 64 KiB of JSON or
text diagnostics for status-aware provider classification.

## Quick Start

```rust
use std::{sync::Arc, time::Duration};

use json_http::{
    JsonHttpClient, JsonHttpSseEvent, ReqwestJsonHttpClient, StaticHeaderAuth,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct DemoRequest {
    prompt: String,
}

#[derive(Deserialize)]
struct DemoResponse {
    ok: bool,
}

async fn call_api() -> json_http::Result<bool> {
    let client = ReqwestJsonHttpClient::new();
    let auth = Arc::new(StaticHeaderAuth::bearer_token("demo-token"));
    let response = client
        .post("https://example.com/v1/demo")
        .auth(auth)
        .json(DemoRequest {
            prompt: "hello".to_owned(),
        })?
        .send::<DemoResponse>()
        .await?;
    Ok(response.body.ok)
}

async fn download_asset() -> json_http::Result<Vec<u8>> {
    let client = ReqwestJsonHttpClient::new();
    let response = client
        .get("https://example.com/v1/assets/video")
        .send_bytes()
        .await?;
    Ok(response.body)
}

async fn next_event() -> json_http::Result<Option<JsonHttpSseEvent>> {
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client
        .post("https://example.com/v1/events")
        .idle_timeout(Duration::from_secs(120))
        .timeout(Duration::from_secs(3_600))
        .json(DemoRequest {
            prompt: "hello".to_owned(),
        })?
        .send_sse()
        .await?;
    stream.next().await
}
```

## Development

```sh
cargo test -p json-http
cargo clippy -p json-http --all-targets --all-features -- -D warnings
```

Downstream tests can enable the `test-support` feature to use the generated
transport, stream, and auth mocks. The integration suite uses a local TCP
server and requires no external network or credentials.

### Key Code

- `src/client.rs` - trait-backed client and transport boundaries
- `src/request.rs` - request builder, request DTOs, and typed response helpers
- `src/sse.rs` - pure incremental SSE framing and stream boundary
- `src/reqwest_sse.rs` - idle/deadline-aware reqwest stream consumption
- `src/reqwest_transport.rs` - buffered and streaming reqwest execution
- `src/auth.rs` - auth hook trait and static header implementation

### Related Docs

- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../ai-models-openai/README.md`](../ai-models-openai/README.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../plans/README.md`](../../plans/README.md)
