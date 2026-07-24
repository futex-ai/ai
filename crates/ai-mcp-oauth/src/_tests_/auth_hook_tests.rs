//! Resource-bound request-authentication tests.

use std::{collections::BTreeMap, sync::Arc};

use json_http::JsonHttpAuth;
use secrecy::SecretString;
use unimock::{MockFn, Unimock, matching};

use crate::{
    CanonicalMcpResource, Error, OAuthCredentialKey, OAuthRequestTokenProvider,
    OAuthRequestTokenProviderMock, OAuthUrlPolicy, RefreshingMcpAuth,
};

#[tokio::test]
async fn inserts_a_fresh_resource_bound_bearer_token() {
    let provider = Arc::new(Unimock::new(
        OAuthRequestTokenProviderMock::token_for_request
            .next_call(matching!(_))
            .returns(Ok(Some(SecretString::from("token-secret".to_owned())))),
    )) as Arc<dyn OAuthRequestTokenProvider>;
    let hook = RefreshingMcpAuth::new(resource(), key(), provider).unwrap();
    let mut headers = BTreeMap::new();

    hook.apply_headers(&mut headers).await.unwrap();

    assert_eq!(headers["Authorization"], "Bearer token-secret");
}

#[tokio::test]
async fn leaves_existing_headers_unchanged_without_credentials() {
    let provider = Arc::new(Unimock::new(
        OAuthRequestTokenProviderMock::token_for_request
            .next_call(matching!(_))
            .returns(Ok(None)),
    )) as Arc<dyn OAuthRequestTokenProvider>;
    let hook = RefreshingMcpAuth::new(resource(), key(), provider).unwrap();
    let mut headers = BTreeMap::from([("x-existing".to_owned(), "value".to_owned())]);

    hook.apply_headers(&mut headers).await.unwrap();

    assert_eq!(
        headers,
        BTreeMap::from([("x-existing".to_owned(), "value".to_owned())])
    );
}

#[test]
fn refuses_a_credential_for_another_resource() {
    let provider = Arc::new(Unimock::new(())) as Arc<dyn OAuthRequestTokenProvider>;
    let other =
        CanonicalMcpResource::parse("https://other.example/mcp", &OAuthUrlPolicy::default())
            .unwrap();

    let error = RefreshingMcpAuth::new(other, key(), provider)
        .err()
        .unwrap();

    assert!(matches!(error, Error::CredentialResourceMismatch));
}

fn resource() -> CanonicalMcpResource {
    CanonicalMcpResource::parse("https://mcp.example/api", &OAuthUrlPolicy::default()).unwrap()
}

fn key() -> OAuthCredentialKey {
    OAuthCredentialKey {
        account_id: "account".to_owned(),
        resource: resource(),
        issuer: "https://auth.example".to_owned(),
        client_id: "client".to_owned(),
        redirect_uri: "https://app.example/callback".to_owned(),
    }
}
