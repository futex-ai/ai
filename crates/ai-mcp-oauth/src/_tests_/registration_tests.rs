//! Public-client registration precedence and mapping tests.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{
    AuthorizationServerMetadata, DefaultOAuthClientRegistry, Error, McpOAuthConfig,
    OAuthClientRegistration, OAuthClientRegistrationSource, OAuthClientRegistry,
    OAuthCredentialStore, OAuthCredentialStoreMock, OAuthHttpResponse, OAuthHttpTransport,
    OAuthHttpTransportMock, OAuthRegistrationRequest, OAuthScopes,
};

#[tokio::test]
async fn configured_registration_precedes_store_and_network() {
    let configured = registration("configured-id", OAuthClientRegistrationSource::Configured);
    let registry = registry(Unimock::new(()), Unimock::new(()));
    let mut request = request();
    request.configured = Some(configured.clone());

    assert_eq!(registry.resolve(&request).await.unwrap(), configured);
}

#[tokio::test]
async fn cached_registration_precedes_dynamic_registration() {
    let cached = registration("cached-id", OAuthClientRegistrationSource::Dynamic);
    let store = Unimock::new(
        OAuthCredentialStoreMock::load_registration
            .next_call(matching!(_))
            .returns(Ok(Some(cached.clone()))),
    );
    let registry = registry(Unimock::new(()), store);

    assert_eq!(registry.resolve(&request()).await.unwrap(), cached);
}

#[tokio::test]
async fn dynamic_registration_maps_public_client_and_refresh_grant() {
    let captured = Arc::new(Mutex::new(None::<Value>));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_json
            .next_call(matching!(_, _, _, _, _))
            .answers_arc({
                let captured = captured.clone();
                Arc::new(move |_, _, _, _, _, body: &Value| {
                    *captured.lock().unwrap() = Some(body.clone());
                    Ok(OAuthHttpResponse {
                        status: 201,
                        headers: Default::default(),
                        body: json!({
                            "client_id": "dynamic-id",
                            "client_secret": "must-be-ignored"
                        }),
                    })
                })
            }),
    );
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_registration
            .next_call(matching!(_))
            .returns(Ok(None)),
        OAuthCredentialStoreMock::save_registration
            .next_call(matching!(_, _))
            .returns(Ok(())),
    ));
    let registration = registry(transport, store)
        .resolve(&request())
        .await
        .unwrap();

    assert_eq!(registration.client_id, "dynamic-id");
    assert_eq!(registration.source, OAuthClientRegistrationSource::Dynamic);
    let body = captured.lock().unwrap().clone().unwrap();
    assert_eq!(
        body["redirect_uris"],
        json!(["https://app.example/callback"])
    );
    assert_eq!(body["response_types"], json!(["code"]));
    assert_eq!(
        body["grant_types"],
        json!(["authorization_code", "refresh_token"])
    );
    assert_eq!(body["token_endpoint_auth_method"], "none");
    assert!(
        !format!("{registration:?}").contains("must-be-ignored"),
        "returned client secret must never enter registration diagnostics"
    );
}

#[tokio::test]
async fn registration_omits_refresh_grant_when_not_advertised() {
    let captured = Arc::new(Mutex::new(None::<Value>));
    let transport = capturing_transport(captured.clone());
    let store = empty_registration_store();
    let mut request = request();
    request.server.grant_types_supported = vec!["authorization_code".to_owned()];

    registry(transport, store).resolve(&request).await.unwrap();

    assert_eq!(
        captured.lock().unwrap().as_ref().unwrap()["grant_types"],
        json!(["authorization_code"])
    );
}

#[tokio::test]
async fn rejects_servers_without_public_client_support() {
    let mut request = request();
    request.server.token_endpoint_auth_methods_supported = vec!["client_secret_basic".to_owned()];

    let error = registry(Unimock::new(()), Unimock::new(()))
        .resolve(&request)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::PublicClientUnsupported));
}

#[tokio::test]
async fn reports_registration_required_without_config_cache_or_dcr() {
    let store = Unimock::new(
        OAuthCredentialStoreMock::load_registration
            .next_call(matching!(_))
            .returns(Ok(None)),
    );
    let mut request = request();
    request.server.registration_endpoint = None;

    let error = registry(Unimock::new(()), store)
        .resolve(&request)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::ClientRegistrationRequired { .. }));
}

#[tokio::test]
async fn registration_rejection_redacts_secret_fields() {
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_json
            .next_call(matching!(_, _, _, _, _))
            .returns(Ok(OAuthHttpResponse {
                status: 400,
                headers: Default::default(),
                body: json!({
                    "error": "invalid_client_metadata",
                    "client_secret": "not-for-diagnostics"
                }),
            })),
    );
    let error = registry(transport, load_none_store())
        .resolve(&request())
        .await
        .unwrap_err();
    let rendered = format!("{error:?}");

    assert!(!rendered.contains("not-for-diagnostics"));
    assert!(rendered.contains("[REDACTED]"));
}

fn registry(transport: Unimock, store: Unimock) -> DefaultOAuthClientRegistry {
    DefaultOAuthClientRegistry::new(
        Arc::new(transport) as Arc<dyn OAuthHttpTransport>,
        Arc::new(store) as Arc<dyn OAuthCredentialStore>,
        McpOAuthConfig::default(),
    )
    .unwrap()
}

fn empty_registration_store() -> Unimock {
    Unimock::new((
        OAuthCredentialStoreMock::load_registration
            .next_call(matching!(_))
            .returns(Ok(None)),
        OAuthCredentialStoreMock::save_registration
            .next_call(matching!(_, _))
            .returns(Ok(())),
    ))
}

fn load_none_store() -> Unimock {
    Unimock::new(
        OAuthCredentialStoreMock::load_registration
            .next_call(matching!(_))
            .returns(Ok(None)),
    )
}

fn capturing_transport(captured: Arc<Mutex<Option<Value>>>) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_json
            .next_call(matching!(_, _, _, _, _))
            .answers_arc(Arc::new(move |_, _, _, _, _, body: &Value| {
                *captured.lock().unwrap() = Some(body.clone());
                Ok(OAuthHttpResponse {
                    status: 201,
                    headers: Default::default(),
                    body: json!({"client_id": "dynamic-id"}),
                })
            })),
    )
}

fn request() -> OAuthRegistrationRequest {
    OAuthRegistrationRequest {
        server: AuthorizationServerMetadata {
            issuer: "https://auth.example".to_owned(),
            authorization_endpoint: "https://auth.example/authorize".to_owned(),
            token_endpoint: "https://auth.example/token".to_owned(),
            registration_endpoint: Some("https://auth.example/register".to_owned()),
            revocation_endpoint: None,
            grant_types_supported: vec![
                "authorization_code".to_owned(),
                "refresh_token".to_owned(),
            ],
            token_endpoint_auth_methods_supported: vec!["none".to_owned()],
            code_challenge_methods_supported: vec!["S256".to_owned()],
            scopes_supported: OAuthScopes::default(),
            unknown: Default::default(),
        },
        redirect_uri: "https://app.example/callback".to_owned(),
        client_name: "Montgomery".to_owned(),
        configured: None,
    }
}

fn registration(client_id: &str, source: OAuthClientRegistrationSource) -> OAuthClientRegistration {
    OAuthClientRegistration {
        client_id: client_id.to_owned(),
        redirect_uri: "https://app.example/callback".to_owned(),
        client_name: "Montgomery".to_owned(),
        source,
    }
}
