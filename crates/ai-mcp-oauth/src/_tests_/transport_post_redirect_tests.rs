//! OAuth production-transport POST redirect tests.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header::LOCATION},
    response::IntoResponse,
    routing::{any, post},
};
use serde_json::json;
use tokio::net::TcpListener;

use crate::{
    Error, OAuthEndpointKind, OAuthHttpLimits, OAuthHttpTransport, OAuthUrlPolicy,
    ReqwestOAuthHttpTransport,
};

#[tokio::test]
async fn post_redirects_never_contact_their_targets() {
    let target_hits = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/form-301",
            post(|| async {
                (
                    StatusCode::MOVED_PERMANENTLY,
                    [(LOCATION, "/redirect-target")],
                )
                    .into_response()
            }),
        )
        .route(
            "/form-302",
            post(|| async {
                (StatusCode::FOUND, [(LOCATION, "/redirect-target")]).into_response()
            }),
        )
        .route(
            "/form-307",
            post(|| async {
                (
                    StatusCode::TEMPORARY_REDIRECT,
                    [(LOCATION, "/redirect-target")],
                )
                    .into_response()
            }),
        )
        .route(
            "/form-303",
            post(|| async {
                (StatusCode::SEE_OTHER, [(LOCATION, "/redirect-target")]).into_response()
            }),
        )
        .route(
            "/json-308",
            post(|| async {
                (
                    StatusCode::PERMANENT_REDIRECT,
                    [(LOCATION, "/redirect-target")],
                )
                    .into_response()
            }),
        )
        .route("/redirect-target", any(record_redirect_target))
        .with_state(target_hits.clone());
    let address = serve(app).await;
    let transport = ReqwestOAuthHttpTransport::new();
    let policy = OAuthUrlPolicy::loopback_development();
    let form = vec![("refresh_token".to_owned(), "secret".to_owned())];

    let moved = transport
        .post_form(
            &format!("http://{address}/form-301"),
            OAuthEndpointKind::Token,
            &policy,
            limits(),
            &form,
        )
        .await;
    let found = transport
        .post_form(
            &format!("http://{address}/form-302"),
            OAuthEndpointKind::Token,
            &policy,
            limits(),
            &form,
        )
        .await;
    let temporary = transport
        .post_form(
            &format!("http://{address}/form-307"),
            OAuthEndpointKind::Token,
            &policy,
            limits(),
            &form,
        )
        .await;
    let see_other = transport
        .post_form(
            &format!("http://{address}/form-303"),
            OAuthEndpointKind::Revocation,
            &policy,
            limits(),
            &form,
        )
        .await;
    let permanent = transport
        .post_json(
            &format!("http://{address}/json-308"),
            OAuthEndpointKind::Registration,
            &policy,
            limits(),
            &json!({"client_name": "secret-adjacent"}),
        )
        .await;

    assert!(matches!(
        moved,
        Err(Error::RedirectNotAllowed {
            endpoint: OAuthEndpointKind::Token
        })
    ));
    assert!(matches!(
        found,
        Err(Error::RedirectNotAllowed {
            endpoint: OAuthEndpointKind::Token
        })
    ));
    assert!(matches!(
        temporary,
        Err(Error::RedirectNotAllowed {
            endpoint: OAuthEndpointKind::Token
        })
    ));
    assert!(matches!(
        see_other,
        Err(Error::RedirectNotAllowed {
            endpoint: OAuthEndpointKind::Revocation
        })
    ));
    assert!(matches!(
        permanent,
        Err(Error::RedirectNotAllowed {
            endpoint: OAuthEndpointKind::Registration
        })
    ));
    assert_eq!(target_hits.load(Ordering::SeqCst), 0);
}

fn limits() -> OAuthHttpLimits {
    OAuthHttpLimits {
        timeout: Duration::from_secs(2),
        max_response_bytes: 1024,
        max_redirects: 2,
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

async fn record_redirect_target(State(hits): State<Arc<AtomicUsize>>) -> Json<serde_json::Value> {
    hits.fetch_add(1, Ordering::SeqCst);
    Json(json!({"replayed": true}))
}
