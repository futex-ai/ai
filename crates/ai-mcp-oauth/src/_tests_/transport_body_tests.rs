//! Production OAuth transport response-body semantics.

use std::time::Duration;

use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};
use tokio::net::TcpListener;

use crate::{
    Error, OAuthEndpointKind, OAuthHttpLimits, OAuthHttpTransport, OAuthUrlPolicy,
    ReqwestOAuthHttpTransport,
};

#[tokio::test]
async fn successful_revocation_ignores_a_plain_text_body() {
    let app = Router::new().route(
        "/revoke",
        post(|| async { (StatusCode::OK, "OK").into_response() }),
    );
    let address = serve(app).await;

    let response = transport()
        .post_form(
            &format!("http://{address}/revoke"),
            OAuthEndpointKind::Revocation,
            &policy(),
            limits(),
            &[],
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, Value::Null);
}

#[tokio::test]
async fn non_json_error_responses_preserve_endpoint_statuses() {
    let app = Router::new()
        .route(
            "/protected",
            get(|| async { (StatusCode::NOT_FOUND, "missing").into_response() }),
        )
        .route(
            "/registration",
            post(|| async { (StatusCode::FORBIDDEN, "denied").into_response() }),
        )
        .route(
            "/token",
            post(|| async { (StatusCode::BAD_REQUEST, "invalid").into_response() }),
        )
        .route(
            "/revocation",
            post(|| async { (StatusCode::SERVICE_UNAVAILABLE, "unavailable").into_response() }),
        );
    let address = serve(app).await;
    let transport = transport();
    let policy = policy();
    let limits = limits();

    let protected = transport
        .get_json(
            &format!("http://{address}/protected"),
            OAuthEndpointKind::ProtectedResourceMetadata,
            &policy,
            limits,
        )
        .await
        .unwrap();
    let registration = transport
        .post_json(
            &format!("http://{address}/registration"),
            OAuthEndpointKind::Registration,
            &policy,
            limits,
            &json!({}),
        )
        .await
        .unwrap();
    let token = transport
        .post_form(
            &format!("http://{address}/token"),
            OAuthEndpointKind::Token,
            &policy,
            limits,
            &[],
        )
        .await
        .unwrap();
    let revocation = transport
        .post_form(
            &format!("http://{address}/revocation"),
            OAuthEndpointKind::Revocation,
            &policy,
            limits,
            &[],
        )
        .await
        .unwrap();

    assert_eq!(
        [
            (protected.status, protected.body),
            (registration.status, registration.body),
            (token.status, token.body),
            (revocation.status, revocation.body),
        ],
        [
            (404, Value::Null),
            (403, Value::Null),
            (400, Value::Null),
            (503, Value::Null),
        ]
    );
}

#[tokio::test]
async fn json_error_responses_remain_available_to_consumers() {
    let app = Router::new().route(
        "/token",
        post(|| async {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid_grant"})),
            )
                .into_response()
        }),
    );
    let address = serve(app).await;

    let response = transport()
        .post_form(
            &format!("http://{address}/token"),
            OAuthEndpointKind::Token,
            &policy(),
            limits(),
            &[],
        )
        .await
        .unwrap();

    assert_eq!(response.status, 400);
    assert_eq!(response.body, json!({"error": "invalid_grant"}));
}

#[tokio::test]
async fn successful_non_revocation_responses_still_require_json() {
    let app = Router::new()
        .route(
            "/protected",
            get(|| async { (StatusCode::OK, "not-json").into_response() }),
        )
        .route(
            "/authorization-server",
            get(|| async { (StatusCode::OK, "not-json").into_response() }),
        )
        .route(
            "/registration",
            post(|| async { (StatusCode::CREATED, "not-json").into_response() }),
        )
        .route(
            "/token",
            post(|| async { (StatusCode::OK, "not-json").into_response() }),
        );
    let address = serve(app).await;
    let transport = transport();
    let policy = policy();
    let limits = limits();

    let protected = transport
        .get_json(
            &format!("http://{address}/protected"),
            OAuthEndpointKind::ProtectedResourceMetadata,
            &policy,
            limits,
        )
        .await;
    let authorization_server = transport
        .get_json(
            &format!("http://{address}/authorization-server"),
            OAuthEndpointKind::AuthorizationServerMetadata,
            &policy,
            limits,
        )
        .await;
    let registration = transport
        .post_json(
            &format!("http://{address}/registration"),
            OAuthEndpointKind::Registration,
            &policy,
            limits,
            &json!({}),
        )
        .await;
    let token = transport
        .post_form(
            &format!("http://{address}/token"),
            OAuthEndpointKind::Token,
            &policy,
            limits,
            &[],
        )
        .await;

    for result in [protected, authorization_server, registration, token] {
        assert!(matches!(result, Err(Error::InvalidJsonResponse)));
    }
}

fn transport() -> ReqwestOAuthHttpTransport {
    ReqwestOAuthHttpTransport::new()
}

fn policy() -> OAuthUrlPolicy {
    OAuthUrlPolicy::loopback_development()
}

fn limits() -> OAuthHttpLimits {
    OAuthHttpLimits {
        timeout: Duration::from_secs(2),
        max_response_bytes: 1024,
        max_redirects: 1,
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
