//! Cross-boundary debug and error secret-redaction coverage.

use secrecy::SecretString;

use crate::{
    Error, OAuthAuthorizationResponse, OAuthScopes, OAuthTokenSet, OAuthTokenType,
    OAuthUserAuthorizationRequest,
};

#[test]
fn debug_and_error_surfaces_never_render_oauth_secrets() {
    let secrets = [
        "access-token-secret",
        "refresh-token-secret",
        "authorization-code-secret",
        "pkce-verifier-secret",
        "callback-state-secret",
        "configured-client-secret",
    ];
    let token = OAuthTokenSet {
        access_token: SecretString::from(secrets[0].to_owned()),
        refresh_token: Some(SecretString::from(secrets[1].to_owned())),
        token_type: OAuthTokenType::Bearer,
        expires_at: Some(100),
        scopes: OAuthScopes::new(["read"]),
    };
    let response = OAuthAuthorizationResponse::authorized(secrets[2], Some(secrets[4]));
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
