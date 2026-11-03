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
- Record sanitized exchanges and replay them without network calls in tests

## What This Crate Does

`json-http` wraps HTTP calls behind `JsonHttpClient`. Requests are built with
`JsonHttpRequestBuilder`, request bodies are serialized from typed Rust structs
or attached as multipart byte fields, and responses can be read as raw bytes,
`serde_json::Value`, or deserialized into typed response DTOs.

Typed request bodies serialize directly into JSON bytes. Object keys keep
their sorted wire order. Buffered responses expose `headers`, and successful
SSE streams expose `headers()`. Both use `JsonHttpHeaders`, a case-insensitive
header map that keeps repeated and binary values. SSE status errors carry the
same headers with their status and bounded body. `Error::is_transport_failure`
identifies network failures without classifying auth or local errors as network
failures.

Failures after headers arrive retain them in `Error::ResponseContext`.
`response_headers()` also reads headers on status errors. `cause()` borrows the
original typed failure, and `into_cause()` returns it by value. Both retain its
progress and diagnostic fields. This covers buffered body reads, typed response
decoding, rejected SSE body reads, and wrong SSE media types. Recordings keep
sanitized headers for these failures and restore their failure class on replay.

Request headers compare without case. `default_headers` adds values after
auth and keeps adapter headers, auth headers, and generated body content types.
Auth hooks run once before dispatch. `credentials_for_origin` limits all
credential headers to the supplied scheme, host, and port.
`auth_for_origin` skips a hook when
the request has a different scheme, host, or port from the supplied origin.
Every header is a credential unless the adapter supplied it through `header`
or `headers` and no auth hook changed it. Standard credential headers always
remain credentials. The builder marks credential names in `auth_headers`.
Requests follow up to ten redirects. Redirects preserve credentials on the
original origin and remove credential headers elsewhere.
The transport also protects standard authorization, cookie, and API-key headers.
It restores credentials only when a redirect returns to the original origin.
Redirect hops share one total timeout budget.
Request and static-auth debug output omit header values.
Error debug output reports the variant, status, header names, and body size.
It omits header values and body text. Stored transport URLs omit user info,
queries, and fragments.

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
- `RecordingTransport`, `MemoryRecordingLog`, and `ReplayTransport` under
  the `test-support` feature for portable exchange recording and replay

`JsonHttpTransport::execute_sse` defaults to the typed `SseUnsupported` error.
Existing alternate transports therefore continue to compile, but must
implement streaming before callers use `send_sse()`. Successful streams require
`text/event-stream`; non-success responses retain at most 64 KiB of JSON or
text diagnostics for status-aware provider classification.

Test support records JSON bodies, base64 response bytes, and decoded SSE
events with their terminal failure class and event count. Multipart requests
store field metadata and byte counts. They do not store uploaded bytes.
Response recordings also keep sanitized headers. Header values use byte arrays
so that repeated and binary values survive replay. Existing recordings without
response headers remain readable and replay with an empty header map.
Credential header names are case-insensitive. The values of `authorization`,
`x-api-key`, `x-goog-api-key`, and `api-key` become `[redacted]`.
The recorder also removes every credential header value that the builder tracks.
The recorder also removes credential URL fields, named JSON credentials,
and known secret values from responses. It forwards the original requests
and responses unchanged. Raw transport error text is never persisted.

The stream boundary does not expose a successful HTTP status. Recordings
therefore use 200 for successful SSE openings. Rejected openings retain
their actual status and body. Streams record only consumed events.
Replay restores transport, idle, deadline, and decode failures with the
observed `events_received` count. A reqwest URL parsing error supplies the
source required by the existing SSE transport error type without using IO.
Replay checks method and URL in call order. `next_response` returns typed
`ReplayError` values for mismatch and exhaustion. Calls through
`JsonHttpTransport` translate these to its existing `Error::Transport`.

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
cargo test -p json-http --all-features
cargo clippy -p json-http --all-targets --all-features -- -D warnings
```

Downstream tests can enable the `test-support` feature to use the generated
transport, stream, and auth mocks. The integration suite uses a local TCP
server and requires no external network or credentials.

### Key Code

- `src/client.rs` - trait-backed client and transport boundaries
- `src/request.rs` - request builder, auth origin scopes, and typed response helpers
- `src/request_types.rs` - serialized request and response DTOs
- `src/canonical/` - typed JSON serialization with sorted object fields
- `src/headers.rs` - common buffered and streaming response header type
- `src/sse.rs` - pure incremental SSE framing and stream boundary
- `src/reqwest_sse.rs` - idle/deadline-aware reqwest stream consumption
- `src/reqwest_transport.rs` - buffered and streaming reqwest execution
- `src/reqwest_redirect.rs` - origin-scoped credentials and the shared redirect budget
- `src/auth.rs` - auth hook trait and static header implementation
- `src/test_support/` - recording types, credential redaction, shared logs,
  transport wrappers, and ordered replay behind `test-support`

### Related Docs

- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../ai-testkit/README.md`](../ai-testkit/README.md)
- [`../ai-models-openai/README.md`](../ai-models-openai/README.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/model-stream-adapters.md`](../../docs/protocol/model-stream-adapters.md)
- [`../../docs/protocol/model-policies.md`](../../docs/protocol/model-policies.md)
