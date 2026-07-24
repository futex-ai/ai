//! OAuth URL syntax and destination-policy tests.

use std::net::{IpAddr, Ipv4Addr};

use crate::{Error, OAuthEndpointKind, OAuthUnsafeUrlReason, OAuthUrlPolicy};

#[test]
fn production_policy_requires_https_and_public_addresses() {
    let policy = OAuthUrlPolicy::default();

    assert!(matches!(
        policy.parse("http://example.com/oauth", OAuthEndpointKind::Authorization),
        Err(Error::UnsafeUrl {
            reason: OAuthUnsafeUrlReason::Scheme,
            ..
        })
    ));
    assert!(matches!(
        policy.parse("https://127.0.0.1/oauth", OAuthEndpointKind::Authorization),
        Err(Error::UnsafeUrl {
            reason: OAuthUnsafeUrlReason::Address,
            ..
        })
    ));
}

#[test]
fn loopback_http_requires_explicit_development_policy() {
    let policy = OAuthUrlPolicy::loopback_development();

    assert!(
        policy
            .parse(
                "http://127.0.0.1:8123/oauth",
                OAuthEndpointKind::Authorization
            )
            .is_ok()
    );
    assert!(!policy.address_allowed(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)), "https"));
}

#[test]
fn rejects_user_info_fragments_and_unsafe_ports() {
    let policy = OAuthUrlPolicy::default();

    for value in [
        "https://user@example.com/oauth",
        "https://example.com/oauth#fragment",
        "https://example.com:22/oauth",
    ] {
        assert!(
            policy
                .parse(value, OAuthEndpointKind::Authorization)
                .is_err(),
            "{value} should be rejected"
        );
    }
}

#[test]
fn alternate_loopback_ip_encoding_is_rejected() {
    let error = OAuthUrlPolicy::default()
        .parse("https://2130706433/oauth", OAuthEndpointKind::Authorization)
        .unwrap_err();

    assert!(matches!(
        error,
        Error::UnsafeUrl {
            reason: OAuthUnsafeUrlReason::Address,
            ..
        }
    ));
}
