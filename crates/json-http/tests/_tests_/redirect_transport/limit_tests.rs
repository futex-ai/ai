//! The existing ten-redirect limit retains its native transport failure.

use std::time::Duration;

use json_http::{Error, JsonHttpClient, ReqwestJsonHttpClient};

use super::support::{Reply, listener, serve};

#[tokio::test]
async fn redirect_limit_retains_reqwest_failure_for_buffered_and_sse() {
    for streaming in [false, true] {
        let (socket, origin) = listener().await;
        let replies = (0..11)
            .map(|_| Reply::redirect(format!("{origin}/again"), "307 Temporary Redirect"))
            .collect();
        let server = serve(socket, replies);
        let builder = ReqwestJsonHttpClient::new()
            .get(&format!("{origin}/start"))
            .timeout(Duration::from_secs(10));
        let error = if streaming {
            match builder.send_sse().await {
                Ok(_) => panic!("redirect loop must fail"),
                Err(error) => error,
            }
        } else {
            builder.send_value().await.unwrap_err()
        };
        let source = match error {
            Error::ReqwestTransport { source } | Error::SseTransport { source, .. } => source,
            error => panic!("native reqwest failure required: {error:?}"),
        };
        assert!(source.is_redirect(), "{source:?}");
        assert_eq!(server.await.unwrap().len(), 11);
    }
}
