//! Cross-boundary debug and error secret-redaction coverage.

use secrecy::{ExposeSecret, SecretString};

use crate::{
    Error, OAuthAuthorizationError, OAuthAuthorizationResponse, OAuthScopes, OAuthTokenSet,
    OAuthTokenType, OAuthUserAuthorizationRequest,
};

#[test]
fn no_state_helpers_wrap_secrets_and_keep_state_absent() {
    let response =
        OAuthAuthorizationResponse::authorized_without_state("authorization-code-secret");
    match response {
        OAuthAuthorizationResponse::Authorized { code, state } => {
            assert_eq!(code.expose_secret(), "authorization-code-secret");
            assert!(state.is_none());
        }
        OAuthAuthorizationResponse::OAuthError { .. } | OAuthAuthorizationResponse::Cancelled => {
            panic!("expected an authorized callback response");
        }
    }

    let response = OAuthAuthorizationResponse::oauth_error_without_state(
        OAuthAuthorizationError::AccessDenied,
    );
    match response {
        OAuthAuthorizationResponse::OAuthError { error, state } => {
            assert_eq!(error, OAuthAuthorizationError::AccessDenied);
            assert!(state.is_none());
        }
        OAuthAuthorizationResponse::Authorized { .. } | OAuthAuthorizationResponse::Cancelled => {
            panic!("expected an OAuth error callback response");
        }
    }
}

#[test]
fn debug_and_error_surfaces_never_render_oauth_secrets() {
    let secrets = [
        "access-token-secret",
        "refresh-token-secret",
        "authorization-code-secret",
        "pkce-verifier-secret",
        "callback-state-secret",
        "configured-client-secret",
        "authorization-code-without-state-secret",
    ];
    let token = OAuthTokenSet {
        access_token: SecretString::from(secrets[0].to_owned()),
        refresh_token: Some(SecretString::from(secrets[1].to_owned())),
        token_type: OAuthTokenType::Bearer,
        expires_at: Some(100),
        scopes: OAuthScopes::new(["read"]),
    };
    let response = OAuthAuthorizationResponse::authorized(secrets[2], Some(secrets[4]));
    let response_without_state = OAuthAuthorizationResponse::authorized_without_state(secrets[6]);
    let error_without_state = OAuthAuthorizationResponse::oauth_error_without_state(
        OAuthAuthorizationError::AccessDenied,
    );
    let request = OAuthUserAuthorizationRequest::new(
        format!(
            "https://auth.example/authorize?code_verifier={}&state={}&client_secret={}",
            secrets[3], secrets[4], secrets[5]
        ),
        100,
    );
    let rendered = [
        format!("{token:?}"),
        format!("{response:?}"),
        format!("{response_without_state:?}"),
        format!("{error_without_state:?}"),
        format!("{request:?}"),
        format!("{:?}", Error::Transport),
        Error::Transport.to_string(),
    ]
    .join("\n");

    for secret in secrets {
        assert!(!rendered.contains(secret), "{secret} was exposed");
    }
    assert!(rendered.contains("[REDACTED]"));
}
