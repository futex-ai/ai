//! OAuth URL syntax and network-destination policy.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

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
        let url = match Url::parse(value) {
            Ok(url) => url,
            Err(_) => return Err(Error::InvalidUrl { endpoint }),
        };
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
        match url.scheme() {
            "https" => {}
            "http" if self.allow_loopback_http && loopback_host => {}
            _ => return Err(unsafe_url(endpoint, OAuthUnsafeUrlReason::Scheme)),
        }
        let port = url.port_or_known_default().unwrap_or(0);
        if port == 0 || (!loopback_host && blocked_port(port)) {
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
        if address.is_loopback() {
            return scheme == "http" && self.allow_loopback_http;
        }
        is_public_address(address)
    }
}

fn is_loopback_host(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(address) => address.is_loopback(),
        Host::Ipv6(address) => address.is_loopback(),
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
        || (octets[0] == 192 && octets[1] == 0)
        || (octets[0] == 198 && matches!(octets[1], 18 | 19))
        || matches!(
            octets,
            [192, 0, 2, _] | [198, 51, 100, _] | [203, 0, 113, _]
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
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

fn unsafe_url(endpoint: OAuthEndpointKind, reason: OAuthUnsafeUrlReason) -> Error {
    Error::UnsafeUrl { endpoint, reason }
}
