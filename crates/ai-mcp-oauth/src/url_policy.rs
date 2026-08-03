//! OAuth URL syntax and network-destination policy.

use std::{
    cell::Cell,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use url::{Host, SyntaxViolation, Url};

use crate::{Error, OAuthEndpointKind, OAuthUnsafeUrlReason, Result};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Security policy applied to discovery, OAuth, and callback URLs.
pub struct OAuthUrlPolicy {
    /// Allows HTTP only when every resolved destination is loopback.
    pub allow_loopback_http: bool,
}

impl OAuthUrlPolicy {
    /// Builds a development policy that additionally permits loopback HTTP.
    pub fn loopback_development() -> Self {
        Self {
            allow_loopback_http: true,
        }
    }

    pub(crate) fn parse(&self, value: &str, endpoint: OAuthEndpointKind) -> Result<Url> {
        let has_user_info = Cell::new(false);
        let record_violation = |violation| {
            if matches!(
                violation,
                SyntaxViolation::EmbeddedCredentials | SyntaxViolation::UnencodedAtSign
            ) {
                has_user_info.set(true);
            }
        };
        let url = match Url::options()
            .syntax_violation_callback(Some(&record_violation))
            .parse(value)
        {
            Ok(url) => url,
            Err(_) => return Err(Error::InvalidUrl { endpoint }),
        };
        if has_user_info.get() {
            return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::UserInfo));
        }
        self.validate_url(&url, endpoint)?;
        Ok(url)
    }

    pub(crate) fn validate_url(&self, url: &Url, endpoint: OAuthEndpointKind) -> Result<()> {
        if !url.username().is_empty() || url.password().is_some() {
            return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::UserInfo));
        }
        if url.fragment().is_some() {
            return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::Fragment));
        }
        let Some(host) = url.host() else {
            return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::MissingHost));
        };
        let loopback_host = is_loopback_host(&host);
        match (url.scheme(), loopback_host) {
            ("https", false) => {}
            ("https", true) => {
                return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::Address));
            }
            ("http", true) if self.allow_loopback_http => {}
            _ => return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::Scheme)),
        }
        let port = url.port_or_known_default().unwrap_or(0);
        if port == 0 || blocked_port(port) {
            return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::Port));
        }
        if let Some(address) = host_address(&host)
            && !self.address_allowed(address, url.scheme())
        {
            return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::Address));
        }
        Ok(())
    }

    pub(crate) fn address_allowed(&self, address: IpAddr, scheme: &str) -> bool {
        match scheme {
            "http" => self.allow_loopback_http && is_loopback_address(address),
            "https" => is_public_address(address),
            _ => false,
        }
    }
}

fn is_loopback_host(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(domain) => is_localhost_domain(domain),
        Host::Ipv4(address) => address.is_loopback(),
        Host::Ipv6(address) => is_loopback_address(IpAddr::V6(*address)),
    }
}

fn is_localhost_domain(domain: &str) -> bool {
    let normalized = domain.strip_suffix('.').unwrap_or(domain);
    normalized.eq_ignore_ascii_case("localhost")
        || normalized
            .rsplit_once('.')
            .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case("localhost"))
}

fn is_loopback_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => address.is_loopback(),
        IpAddr::V6(address) => {
            address.is_loopback()
                || address
                    .to_ipv4_mapped()
                    .is_some_and(|mapped| mapped.is_loopback())
        }
    }
}

fn host_address(host: &Host<&str>) -> Option<IpAddr> {
    match host {
        Host::Domain(_) => None,
        Host::Ipv4(address) => Some(IpAddr::V4(*address)),
        Host::Ipv6(address) => Some(IpAddr::V6(*address)),
    }
}

fn blocked_port(port: u16) -> bool {
    matches!(
        port,
        20 | 21
            | 22
            | 23
            | 25
            | 53
            | 69
            | 110
            | 111
            | 135
            | 137
            | 139
            | 143
            | 161
            | 389
            | 445
            | 512
            | 513
            | 514
            | 2049
            | 2375
            | 3306
            | 5432
            | 6379
            | 11211
    )
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_v4(address),
        IpAddr::V6(address) => is_public_v6(address),
    }
}

fn is_public_v4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    !(address.is_unspecified()
        || address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_multicast()
        || address.is_broadcast()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || matches!(octets, [192, 0, 0, last] if !matches!(last, 9 | 10))
        || (octets[0] == 198 && matches!(octets[1], 18 | 19))
        || matches!(
            octets,
            [192, 0, 2, _] | [192, 88, 99, _] | [198, 51, 100, _] | [203, 0, 113, _]
        )
        || octets[0] >= 240)
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_v4(mapped);
    }
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_multicast()
        || is_ipv4_transition_address(segments)
        || is_special_purpose_v6(segments)
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0)
}

/// Detects IPv4-compatible, NAT64, and 6to4 address ranges.
fn is_ipv4_transition_address(segments: [u16; 8]) -> bool {
    segments[..6].iter().all(|segment| *segment == 0)
        || (segments[0] == 0x0064
            && segments[1] == 0xff9b
            && segments[2..6].iter().all(|segment| *segment == 0))
        || (segments[0] == 0x0064 && segments[1] == 0xff9b && segments[2] == 0x0001)
        || segments[0] == 0x2002
}

/// Detects the remaining non-global IANA special-purpose IPv6 ranges.
///
/// Covers discard and dummy `/64` prefixes, the IETF protocol-assignment
/// `/23`, both documentation prefixes, and the SRv6 SID `/16`.
fn is_special_purpose_v6(segments: [u16; 8]) -> bool {
    (segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && matches!(segments[3], 0 | 1))
        || (segments[0] == 0x2001 && (segments[1] & 0xfe00) == 0)
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || (segments[0] == 0x3fff && (segments[1] & 0xf000) == 0)
        || segments[0] == 0x5f00
}

fn unsafe_url(endpoint: OAuthEndpointKind, reason: OAuthUnsafeUrlReason) -> Error {
    Error::UnsafeUrl { endpoint, reason }
}
