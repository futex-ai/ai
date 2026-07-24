//! Deterministic PKCE and secret-redaction tests.

use unimock::{MockFn, Unimock, matching};

use crate::{
    OAuthAuthorizationResponse, OAuthRandomMock,
    pkce::{generate_authorization_secrets, pkce_challenge},
};

#[test]
fn matches_the_rfc_7636_s256_vector() {
    assert_eq!(
        pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn generates_independent_43_character_state_and_verifier() {
    let random = Unimock::new((
        OAuthRandomMock::bytes
            .next_call(matching!(32))
            .returns(Ok(vec![1; 32])),
        OAuthRandomMock::bytes
            .next_call(matching!(32))
            .returns(Ok(vec![2; 32])),
    ));

    let secrets = generate_authorization_secrets(&random).unwrap();

    use secrecy::ExposeSecret;
    assert_eq!(secrets.state.expose_secret().len(), 43);
    assert_eq!(secrets.verifier.expose_secret().len(), 43);
    assert_ne!(
        secrets.state.expose_secret(),
        secrets.verifier.expose_secret()
    );
}

#[test]
fn callback_debug_output_redacts_code_and_state() {
    let response =
        OAuthAuthorizationResponse::authorized("authorization-code-secret", Some("state-secret"));
    let rendered = format!("{response:?}");

    assert!(!rendered.contains("authorization-code-secret"));
    assert!(!rendered.contains("state-secret"));
    assert!(rendered.contains("[REDACTED]"));
}
