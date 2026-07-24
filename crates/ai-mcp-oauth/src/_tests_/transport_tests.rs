//! Production OAuth transport bounds, redirects, and DNS policy tests.

use std::{net::IpAddr, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    http::{HeaderValue, StatusCode, header::LOCATION},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;
use tokio::net::TcpListener;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, OAuthDnsResolver, OAuthDnsResolverMock, OAuthEndpointKind, OAuthHttpLimits,
    OAuthHttpTransport, OAuthUrlPolicy, ReqwestOAuthHttpTransport,
};

#[tokio::test]
async fn follows_a_validated_loopback_redirect() {
    let app = Router::new()
        .route(
            "/start",
            get(|| async {
                let mut response = Response::new(axum::body::Body::empty());
                *response.status_mut() = StatusCode::FOUND;
                response
                    .headers_mut()
                    .insert(LOCATION, HeaderValue::from_static("/final"));
                response
            }),
        )
        .route("/final", get(|| async { Json(json!({"ok": true})) }));
    let address = serve(app).await;
    let response = ReqwestOAuthHttpTransport::new()
        .get_json(
            &format!("http://{address}/start"),
            OAuthEndpointKind::ProtectedResourceMetadata,
            &OAuthUrlPolicy::loopback_development(),
            limits(2, 1024),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, json!({"ok": true}));
}

#[tokio::test]
async fn caps_response_bytes_while_streaming() {
    let app = Router::new().route(
        "/large",
        get(|| async { Json(json!({"value": "x".repeat(256)})) }),
    );
    let address = serve(app).await;
    let error = ReqwestOAuthHttpTransport::new()
        .get_json(
            &format!("http://{address}/large"),
            OAuthEndpointKind::ProtectedResourceMetadata,
            &OAuthUrlPolicy::loopback_development(),
            limits(1, 32),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::ResponseTooLarge { limit_bytes: 32 }));
}

#[tokio::test]
async fn rejects_an_unsafe_redirect_before_dispatch() {
    let app = Router::new().route(
        "/unsafe",
        get(|| async {
            (
                StatusCode::FOUND,
                [(LOCATION, "http://169.254.169.254/latest")],
            )
                .into_response()
        }),
    );
    let address = serve(app).await;
    let error = ReqwestOAuthHttpTransport::new()
        .get_json(
            &format!("http://{address}/unsafe"),
            OAuthEndpointKind::ProtectedResourceMetadata,
            &OAuthUrlPolicy::loopback_development(),
            limits(1, 1024),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::UnsafeUrl { .. }));
}

#[tokio::test]
async fn rejects_every_resolution_when_one_address_is_unsafe() {
    let resolver = Arc::new(Unimock::new(
        OAuthDnsResolverMock::resolve
            .next_call(matching!("oauth.example", 443))
            .returns(Ok(vec![
                "93.184.216.34".parse::<IpAddr>().unwrap(),
                "127.0.0.1".parse::<IpAddr>().unwrap(),
            ])),
    )) as Arc<dyn OAuthDnsResolver>;
    let transport = ReqwestOAuthHttpTransport::with_resolver(resolver);

    let error = transport
        .get_json(
            "https://oauth.example/metadata",
            OAuthEndpointKind::AuthorizationServerMetadata,
            &OAuthUrlPolicy::default(),
            limits(1, 1024),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::UnsafeUrl { .. }));
}

fn limits(max_redirects: usize, max_response_bytes: usize) -> OAuthHttpLimits {
    OAuthHttpLimits {
        timeout: Duration::from_secs(2),
        max_response_bytes,
        max_redirects,
    }
}

async fn serve(app: Router) -> std::net::SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}
