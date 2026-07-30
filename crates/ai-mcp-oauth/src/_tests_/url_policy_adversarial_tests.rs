//! Adversarial OAuth URL syntax and address-policy coverage.

use std::net::{IpAddr, Ipv4Addr};

use crate::{OAuthEndpointKind, OAuthUrlPolicy};

#[test]
fn rejects_dangerous_schemes_user_info_and_fragments() {
    let policy = OAuthUrlPolicy::default();
    for value in [
        "ftp://example.com/oauth",
        "file:///tmp/token",
        "javascript:alert(1)",
        "data:text/plain,token",
        "https://user:password@example.com/oauth",
        "https://example.com/oauth#secret",
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
fn rejects_private_reserved_link_local_and_metadata_addresses() {
    let policy = OAuthUrlPolicy::default();
    for value in [
        "https://0.0.0.0/oauth",
        "https://10.0.0.1/oauth",
        "https://100.64.0.1/oauth",
        "https://127.0.0.1/oauth",
        "https://169.254.169.254/latest/meta-data",
        "https://172.16.0.1/oauth",
        "https://192.168.0.1/oauth",
        "https://192.0.2.1/oauth",
        "https://198.18.0.1/oauth",
        "https://224.0.0.1/oauth",
        "https://240.0.0.1/oauth",
        "https://[::1]/oauth",
        "https://[fc00::1]/oauth",
        "https://[fe80::1]/oauth",
        "https://[fec0::1]/oauth",
        "https://[2001:db8::1]/oauth",
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
fn rejects_alternate_loopback_encodings_and_sensitive_public_ports() {
    let policy = OAuthUrlPolicy::default();
    for value in [
        "https://2130706433/oauth",
        "https://0177.0.0.1/oauth",
        "https://0x7f000001/oauth",
        "https://example.com:22/oauth",
        "https://example.com:6379/oauth",
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
fn permits_loopback_http_only_under_explicit_development_policy() {
    let production = OAuthUrlPolicy::default();
    let development = OAuthUrlPolicy::loopback_development();
    let endpoint = "http://127.0.0.1:8123/authorize";

    assert!(
        production
            .parse(endpoint, OAuthEndpointKind::Authorization)
            .is_err()
    );
    assert!(
        development
            .parse(endpoint, OAuthEndpointKind::Authorization)
            .is_ok()
    );
}

#[test]
fn permits_public_addresses_outside_ietf_protocol_assignments() {
    let policy = OAuthUrlPolicy::default();
    for value in ["192.0.1.1", "192.0.3.1", "192.0.255.255"] {
        let address = value.parse::<Ipv4Addr>().unwrap();
        assert!(
            policy.address_allowed(IpAddr::V4(address), "https"),
            "{value} should be globally reachable"
        );
        assert!(
            policy
                .parse(
                    &format!("https://{value}/oauth"),
                    OAuthEndpointKind::Authorization
                )
                .is_ok(),
            "{value} should be accepted as an HTTPS literal"
        );
    }
}

#[test]
fn permits_global_anycast_protocol_assignment_exceptions() {
    let policy = OAuthUrlPolicy::default();
    for value in ["192.0.0.9", "192.0.0.10"] {
        let address = value.parse::<Ipv4Addr>().unwrap();
        assert!(
            policy.address_allowed(IpAddr::V4(address), "https"),
            "{value} should be globally reachable"
        );
        assert!(
            policy
                .parse(
                    &format!("https://{value}/oauth"),
                    OAuthEndpointKind::Authorization
                )
                .is_ok(),
            "{value} should be accepted as an HTTPS literal"
        );
    }
}

#[test]
fn rejects_non_global_protocol_assignments_and_test_net() {
    let policy = OAuthUrlPolicy::default();
    for value in [
        "192.0.0.0",
        "192.0.0.8",
        "192.0.0.11",
        "192.0.0.170",
        "192.0.0.171",
        "192.0.0.255",
        "192.0.2.1",
    ] {
        let address = value.parse::<Ipv4Addr>().unwrap();
        assert!(
            !policy.address_allowed(IpAddr::V4(address), "https"),
            "{value} should remain non-global"
        );
        assert!(
            policy
                .parse(
                    &format!("https://{value}/oauth"),
                    OAuthEndpointKind::Authorization
                )
                .is_err(),
            "{value} should remain rejected as an HTTPS literal"
        );
    }
}

#[test]
fn ipv4_mapped_ipv6_uses_the_corrected_ipv4_boundary() {
    let policy = OAuthUrlPolicy::default();
    for value in ["192.0.1.1", "192.0.0.9"] {
        let mapped = value.parse::<Ipv4Addr>().unwrap().to_ipv6_mapped();
        assert!(
            policy.address_allowed(IpAddr::V6(mapped), "https"),
            "{mapped} should be globally reachable"
        );
        assert!(
            policy
                .parse(
                    &format!("https://[{mapped}]/oauth"),
                    OAuthEndpointKind::Authorization
                )
                .is_ok(),
            "{mapped} should be accepted as an HTTPS literal"
        );
    }
    for value in ["192.0.0.8", "192.0.2.1"] {
        let mapped = value.parse::<Ipv4Addr>().unwrap().to_ipv6_mapped();
        assert!(
            !policy.address_allowed(IpAddr::V6(mapped), "https"),
            "{mapped} should remain non-global"
        );
        assert!(
            policy
                .parse(
                    &format!("https://[{mapped}]/oauth"),
                    OAuthEndpointKind::Authorization
                )
                .is_err(),
            "{mapped} should remain rejected as an HTTPS literal"
        );
    }
}
