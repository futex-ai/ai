# json-http

`json-http` is a small typed HTTP client crate for workspace services and
provider clients. Depend on it when you need a reusable builder-style API over
JSON requests, multipart uploads, or authenticated binary downloads without
embedding reqwest boilerplate in each caller.

## Responsibilities

- Provide a trait-backed HTTP client boundary for JSON, multipart, and binary
  response requests
- Expose a builder API for `get`, `post`, `put`, `delete`, and `patch`
- Support reusable auth hooks that apply request headers before dispatch

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
- `JsonHttpMultipartField` for small multipart upload calls that still use the
  same auth hooks and response handling
- per-request transport timeouts with a 60-second default
- `send_bytes()` for authenticated downloads through the same transport and
  auth hooks as JSON calls

## Quick Start

```rust
use std::sync::Arc;

use json_http::{JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth};
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
```

## Development

```sh
cargo test -p json-http
cargo clippy -p json-http --all-targets --all-features -- -D warnings
```

Downstream tests can enable the `test-support` feature to use the generated
transport and auth mocks.

### Key Code

- `src/client.rs` - trait-backed client types and reqwest transport
- `src/request.rs` - request builder, request DTOs, and typed response helpers
- `src/auth.rs` - auth hook trait and static header implementation

### Related Docs

- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../ai-models-openai/README.md`](../ai-models-openai/README.md)
- [`../../plans/README.md`](../../plans/README.md)
