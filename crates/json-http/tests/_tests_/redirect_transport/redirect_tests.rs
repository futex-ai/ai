//! Redirect credentials stay on their original origin for every response mode.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use super::support::{CountingAuth, Reply, listener, serve};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient, StaticHeaderAuth};
use serde_json::json;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Json,
    Bytes,
    Sse,
}

#[tokio::test]
async fn redirect_hops_keep_auth_only_on_the_original_origin() {
    for mode in [Mode::Json, Mode::Bytes, Mode::Sse] {
        let (origin_listener, origin) = listener().await;
        let (foreign_listener, foreign) = listener().await;
        let (content_type, body, status, redirected_method) = match mode {
            Mode::Json => (
                "application/json",
                "{\"ok\":true}",
                "308 Permanent Redirect",
                "POST",
            ),
            Mode::Bytes => ("application/octet-stream", "asset", "302 Found", "GET"),
            Mode::Sse => (
                "text/event-stream",
                "data: done\n\n",
                "303 See Other",
                "GET",
            ),
        };
        let origin_server = serve(
            origin_listener,
            vec![
                Reply::redirect(format!("{origin}/same"), "307 Temporary Redirect"),
                Reply::redirect(format!("{foreign}/foreign"), status),
                Reply::success(content_type, body),
            ],
        );
        let foreign_server = serve(
            foreign_listener,
            vec![
                Reply::redirect(format!("{foreign}/next"), "307 Temporary Redirect"),
                Reply::redirect(format!("{origin}/return"), "307 Temporary Redirect"),
            ],
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let auth = Arc::new(CountingAuth {
            calls: calls.clone(),
        });
        let client = ReqwestJsonHttpClient::new();
        let builder = client
            .post(&format!("{origin}/start"))
            .timeout(Duration::from_secs(10))
            .default_headers(BTreeMap::from([(
                "CONTENT-TYPE".to_owned(),
                "extra-content".to_owned(),
            )]))
            .default_headers(BTreeMap::from([
                ("X-Session-Key".to_owned(), "hook-secret".to_owned()),
                ("x-gateway-key".to_owned(), "gateway-secret".to_owned()),
            ]))
            .header("x-public", "visible")
            .header("x-changed", "public-before")
            .auth(Arc::new(StaticHeaderAuth::new(BTreeMap::from([
                ("x-api-key".to_owned(), "api-secret".to_owned()),
                (
                    "Authorization".to_owned(),
                    "Bearer bearer-secret".to_owned(),
                ),
            ]))))
            .auth(auth)
            .json(json!({"request":true}))
            .unwrap();
        match mode {
            Mode::Json => assert_eq!(builder.send_value().await.unwrap().body, json!({"ok":true})),
            Mode::Bytes => assert_eq!(builder.send_bytes().await.unwrap().body, b"asset"),
            Mode::Sse => {
                let mut stream = builder.send_sse().await.unwrap();
                assert_eq!(stream.next().await.unwrap().unwrap().data, "done");
                assert_eq!(stream.next().await.unwrap(), None);
            }
        }
        let original = origin_server.await.unwrap();
        let redirected = foreign_server.await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        for request in &redirected {
            assert_eq!(request.headers.get("x-api-key"), None, "{mode:?}");
            assert_eq!(request.headers.get("authorization"), None, "{mode:?}");
            assert_eq!(request.headers.get("x-session-key"), None, "{mode:?}");
            assert_eq!(request.headers.get("x-changed"), None, "{mode:?}");
            assert_eq!(request.headers.get("x-gateway-key"), None, "{mode:?}");
            assert_eq!(
                request.headers.get("x-public").map(String::as_str),
                Some("visible")
            );
            assert_eq!(request.method, redirected_method, "{mode:?}");
        }
        for (index, request) in original.iter().enumerate() {
            assert_eq!(
                request.headers.get("x-gateway-key").map(String::as_str),
                Some("gateway-secret")
            );
            assert_eq!(
                request.headers.get("x-api-key").map(String::as_str),
                Some("api-secret"),
                "{mode:?} origin hop {index}"
            );
            assert_eq!(
                request.headers.get("authorization").map(String::as_str),
                Some("Bearer bearer-secret")
            );
            assert_eq!(
                request.headers.get("x-session-key").map(String::as_str),
                Some("hook-secret")
            );
            assert_eq!(
                request.headers.get("x-changed").map(String::as_str),
                Some("changed-secret")
            );
            assert_eq!(
                request.headers.get("x-public").map(String::as_str),
                Some("visible")
            );
        }
        assert_eq!(original[0].method, "POST");
        assert_eq!(original[1].method, "POST");
        assert_eq!(original[0].body, b"{\"request\":true}");
        assert_eq!(original[1].body, original[0].body);
        assert_eq!(
            original[0].headers.get("content-type").map(String::as_str),
            Some("application/json")
        );
        assert_eq!(original[2].method, redirected_method, "{mode:?}");
        let expected_body = if redirected_method == "POST" {
            b"{\"request\":true}".as_slice()
        } else {
            b""
        };
        assert_eq!(redirected[0].body, expected_body, "{mode:?}");
        assert_eq!(original[2].body, expected_body, "{mode:?}");
    }
}
