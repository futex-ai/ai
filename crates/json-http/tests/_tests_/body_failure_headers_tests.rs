//! Headers survive response-body failures after the server has sent them.

use std::time::Duration;

use json_http::{Error, JsonHttpClient, ReqwestJsonHttpClient};

use super::support::{BodyFraming, ResponseSpec, ResponseStep, spawn_server};

#[tokio::test]
async fn buffered_and_rejected_stream_body_failures_keep_headers() {
    for path in ["json", "bytes", "sse"] {
        let body = b"truncated".to_vec();
        let server = spawn_server(ResponseSpec {
            status: if path == "sse" {
                "429 Too Many Requests"
            } else {
                "200 OK"
            },
            content_type: Some("application/json\r\nX-Request-Id: req-body-read"),
            header_delay: Duration::ZERO,
            framing: BodyFraming::Fixed {
                declared_length: body.len() + 100,
            },
            steps: vec![ResponseStep::Bytes(body)],
        })
        .await;
        let client = ReqwestJsonHttpClient::new();
        let error = match path {
            "json" => client.get(&server.url).send_value().await.err(),
            "bytes" => client.get(&server.url).send_bytes().await.err(),
            "sse" => client.get(&server.url).send_sse().await.err(),
            _ => unreachable!(),
        }
        .expect("truncated response should fail");
        assert_eq!(
            error
                .response_headers()
                .and_then(|headers| headers.get("x-request-id"))
                .and_then(|value| value.to_str().ok()),
            Some("req-body-read"),
            "{path}"
        );
        assert!(
            matches!(
                error.cause(),
                Error::ReqwestTransport { .. }
                    | Error::SseTransport {
                        events_received: 0,
                        ..
                    }
            ),
            "{path}: {error:?}"
        );
        assert!(error.is_transport_failure(), "{path}: {error:?}");
        server.request().await;
    }
}
