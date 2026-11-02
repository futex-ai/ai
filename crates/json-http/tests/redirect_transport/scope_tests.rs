//! Auth hooks also respect the origin of the first download URL.

use std::{sync::Arc, time::Duration};

use json_http::{JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth};

use super::support::{Reply, listener, serve};

#[tokio::test]
async fn origin_scoped_hook_is_skipped_for_a_different_initial_origin() {
    let (socket, origin) = listener().await;
    let server = serve(
        socket,
        vec![Reply::success("application/octet-stream", "asset")],
    );
    let response = ReqwestJsonHttpClient::new()
        .get(&format!("{origin}/asset"))
        .timeout(Duration::from_secs(10))
        .auth_for_origin(
            Arc::new(StaticHeaderAuth::bearer_token("api-secret")),
            "http://127.0.0.1:1",
        )
        .send_bytes()
        .await
        .unwrap();
    assert_eq!(response.body, b"asset");
    let requests = server.await.unwrap();
    assert_eq!(requests[0].headers.get("authorization"), None);
}
