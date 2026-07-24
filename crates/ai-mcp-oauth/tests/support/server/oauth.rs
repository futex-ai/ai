//! OAuth discovery, registration, authorization, token, and revocation routes.

use std::{collections::BTreeMap, sync::Arc};

use axum::{
    Form, Json,
    extract::{Query, State},
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, LOCATION},
    },
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use url::Url;

use super::ServerState;

pub(super) async fn protected_resource(State(state): State<Arc<ServerState>>) -> Response {
    json_response(json!({
        "resource": format!("{}/mcp", state.base_url),
        "authorization_servers": [&state.base_url],
        "scopes_supported": ["read", "write"]
    }))
}

pub(super) async fn authorization_server(State(state): State<Arc<ServerState>>) -> Response {
    json_response(json!({
        "issuer": &state.base_url,
        "authorization_endpoint": format!("{}/authorize", state.base_url),
        "token_endpoint": format!("{}/token", state.base_url),
        "registration_endpoint": format!("{}/register", state.base_url),
        "revocation_endpoint": format!("{}/revoke", state.base_url),
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_methods_supported": ["none"],
        "code_challenge_methods_supported": ["S256"],
        "scopes_supported": ["read", "write"]
    }))
}

pub(super) async fn register(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Response {
    state.records.lock().await.registrations.push(body);
    (StatusCode::CREATED, Json(json!({"client_id": "client-id"}))).into_response()
}

pub(super) async fn authorize(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    state
        .records
        .lock()
        .await
        .authorization_queries
        .push(query.clone());
    let behavior = state.behavior.lock().await.clone();
    let mut callback = Url::parse(query.get("redirect_uri").unwrap()).unwrap();
    callback
        .query_pairs_mut()
        .append_pair("state", query.get("state").unwrap())
        .append_pair(
            if behavior.deny_authorization {
                "error"
            } else {
                "code"
            },
            if behavior.deny_authorization {
                "access_denied"
            } else {
                "authorization-code"
            },
        );
    let mut response = StatusCode::FOUND.into_response();
    response
        .headers_mut()
        .insert(LOCATION, HeaderValue::from_str(callback.as_str()).unwrap());
    response
}

pub(super) async fn token(
    State(state): State<Arc<ServerState>>,
    Form(form): Form<BTreeMap<String, String>>,
) -> Response {
    state.records.lock().await.token_forms.push(form.clone());
    let behavior = state.behavior.lock().await.clone();
    if form.get("grant_type").map(String::as_str) == Some("refresh_token") {
        if !behavior.refresh_delay.is_zero() {
            tokio::time::sleep(behavior.refresh_delay).await;
        }
        if behavior.refresh_invalid_grant {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid_grant"})),
            )
                .into_response();
        }
        return Json(json!({
            "access_token": "access-2",
            "refresh_token": "refresh-2",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read"
        }))
        .into_response();
    }
    let access_token = if behavior.granted_scope.is_some() {
        "access-scope"
    } else {
        "access-1"
    };
    let mut body = json!({
        "access_token": access_token,
        "refresh_token": "refresh-1",
        "token_type": "Bearer",
        "expires_in": 3600
    });
    if let Some(scope) = behavior.granted_scope {
        body["scope"] = Value::String(scope);
    }
    Json(body).into_response()
}

pub(super) async fn revoke(
    State(state): State<Arc<ServerState>>,
    Form(form): Form<BTreeMap<String, String>>,
) -> Response {
    state.records.lock().await.revocation_forms.push(form);
    StatusCode::OK.into_response()
}

fn json_response(body: Value) -> Response {
    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("max-age=60"));
    response
}
