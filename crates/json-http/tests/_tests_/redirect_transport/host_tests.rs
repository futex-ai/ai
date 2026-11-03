//! Credentials stay private when only the redirect host changes.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use json_http::{JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth};
use serde_json::json;
use tokio::time::timeout;

use super::support::{CountingAuth, Reply, listener, serve};

#[tokio::test]
async fn same_port_cross_host_redirect_omits_every_auth_header() {
    let (socket, origin) = listener().await;
    let port = socket.local_addr().unwrap().port();
    let destination = format!("http://localhost:{port}");
    let server = serve(
        socket,
        vec![
            Reply::redirect(format!("{destination}/finish"), "307 Temporary Redirect"),
            Reply::success("application/json", "{\"ok\":true}"),
        ],
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let response = ReqwestJsonHttpClient::new()
        .get(&format!("{origin}/start"))
        .timeout(Duration::from_secs(10))
        .header("x-public", "visible")
        .header("x-changed", "public-before")
        .auth(Arc::new(StaticHeaderAuth::new(BTreeMap::from([
            ("x-api-key".to_owned(), "api-secret".to_owned()),
            ("x-goog-api-key".to_owned(), "google-secret".to_owned()),
            (
                "Authorization".to_owned(),
                "Bearer bearer-secret".to_owned(),
            ),
        ]))))
        .auth(Arc::new(CountingAuth {
            calls: calls.clone(),
        }))
        .send_value()
        .await
        .unwrap();
    let requests = timeout(Duration::from_secs(10), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, json!({"ok":true}));
    assert_eq!(requests.len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        requests[0].headers.get("host"),
        Some(&format!("127.0.0.1:{port}"))
    );
    assert_eq!(
        requests[1].headers.get("host"),
        Some(&format!("localhost:{port}"))
    );
    for (header, value) in [
        ("x-api-key", "api-secret"),
        ("x-goog-api-key", "google-secret"),
        ("authorization", "Bearer bearer-secret"),
        ("x-session-key", "hook-secret"),
        ("x-changed", "changed-secret"),
    ] {
        assert_eq!(
            requests[0].headers.get(header).map(String::as_str),
            Some(value),
            "initial {header}"
        );
        assert_eq!(requests[1].headers.get(header), None, "redirected {header}");
    }
    for request in requests {
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.headers.get("x-public").map(String::as_str),
            Some("visible")
        );
    }
}
