//! OAuth URL syntax and destination-policy tests.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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

#[test]
fn production_rejects_every_syntactic_loopback_host() {
    let policy = OAuthUrlPolicy::default();

    for value in [
        "https://localhost/oauth",
        "https://LOCALHOST/oauth",
        "https://localhost./oauth",
        "https://api.localhost/oauth",
        "https://API.LOCALHOST/oauth",
    ] {
        assert!(matches!(
            policy.parse(value, OAuthEndpointKind::Authorization),
            Err(Error::UnsafeUrl {
                reason: OAuthUnsafeUrlReason::Address,
                ..
            })
        ));
    }

    assert!(
        policy
            .parse(
                "https://localhost.evil.example/oauth",
                OAuthEndpointKind::Authorization
            )
            .is_ok()
    );
}

#[test]
fn development_http_accepts_only_syntactic_loopback_hosts() {
    let policy = OAuthUrlPolicy::loopback_development();

    for value in [
        "http://localhost:8123/oauth",
        "http://LocalHost:8123/oauth",
        "http://localhost.:8123/oauth",
        "http://api.localhost:8123/oauth",
        "http://127.8.9.10:8123/oauth",
        "http://[::1]:8123/oauth",
        "http://[::ffff:127.0.0.1]:8123/oauth",
    ] {
        assert!(
            policy
                .parse(value, OAuthEndpointKind::Authorization)
                .is_ok(),
            "{value} should be accepted"
        );
    }

    for value in [
        "http://example.com/oauth",
        "http://localhost.evil.example/oauth",
    ] {
        assert!(matches!(
            policy.parse(value, OAuthEndpointKind::Authorization),
            Err(Error::UnsafeUrl {
                reason: OAuthUnsafeUrlReason::Scheme,
                ..
            })
        ));
    }
}

#[test]
fn loopback_hosts_do_not_bypass_scheme_or_port_policy() {
    let policy = OAuthUrlPolicy::loopback_development();

    assert!(matches!(
        policy.parse(
            "https://localhost:8123/oauth",
            OAuthEndpointKind::Authorization
        ),
        Err(Error::UnsafeUrl {
            reason: OAuthUnsafeUrlReason::Address,
            ..
        })
    ));
    for value in ["http://localhost:6379/oauth", "http://127.0.0.1:22/oauth"] {
        assert!(matches!(
            policy.parse(value, OAuthEndpointKind::Authorization),
            Err(Error::UnsafeUrl {
                reason: OAuthUnsafeUrlReason::Port,
                ..
            })
        ));
    }
}

#[test]
fn development_http_requires_a_resolved_loopback_address() {
    let policy = OAuthUrlPolicy::loopback_development();
    let public = IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34));
    let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let mapped_loopback = IpAddr::V6(Ipv6Addr::from(0xffff_7f00_0001_u128));

    assert!(policy.address_allowed(loopback, "http"));
    assert!(policy.address_allowed(mapped_loopback, "http"));
    assert!(!policy.address_allowed(loopback, "https"));
    assert!(!policy.address_allowed(public, "http"));
    assert!(policy.address_allowed(public, "https"));
}

#[test]
fn production_rejects_ipv4_transition_ipv6_ranges() {
    let policy = OAuthUrlPolicy::default();
    let addresses = [
        Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0x7f00, 1),
        Ipv6Addr::new(0x0064, 0xff9b, 0, 0, 0, 0, 0x7f00, 1),
        Ipv6Addr::new(0x2002, 0x7f00, 1, 0, 0, 0, 0, 0),
    ];

    for address in addresses {
        assert!(
            !policy.address_allowed(IpAddr::V6(address), "https"),
            "{address} should be rejected"
        );
    }
}
