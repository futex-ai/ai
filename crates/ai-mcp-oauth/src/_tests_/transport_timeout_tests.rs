//! OAuth production-transport timeout tests.

use std::{future, net::IpAddr, sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::{
    Error, OAuthConfigField, OAuthDnsResolver, OAuthEndpointKind, OAuthHttpLimits,
    OAuthHttpTransport, OAuthUrlPolicy, ReqwestOAuthHttpTransport, Result,
};

#[tokio::test(start_paused = true)]
async fn stalled_dns_resolution_cannot_outlive_one_hop_timeout() {
    let hop_timeout = Duration::from_secs(5);
    let transport = ReqwestOAuthHttpTransport::with_resolver(Arc::new(StalledResolver));
    let policy = OAuthUrlPolicy::default();
    let started_at = tokio::time::Instant::now();
    let request = transport.get_json(
        "https://oauth.example/metadata",
        OAuthEndpointKind::AuthorizationServerMetadata,
        &policy,
        OAuthHttpLimits {
            timeout: hop_timeout,
            max_response_bytes: 1024,
            max_redirects: 1,
        },
    );

    let bounded = tokio::time::timeout(Duration::from_secs(6), request).await;

    assert!(matches!(bounded, Ok(Err(Error::Dns))));
    assert_eq!(started_at.elapsed(), hop_timeout);
}

#[tokio::test]
async fn unrepresentable_hop_deadline_returns_a_typed_error() {
    let result = ReqwestOAuthHttpTransport::new()
        .get_json(
            "http://127.0.0.1:9/metadata",
            OAuthEndpointKind::ProtectedResourceMetadata,
            &OAuthUrlPolicy::loopback_development(),
            OAuthHttpLimits {
                timeout: Duration::MAX,
                max_response_bytes: 1024,
                max_redirects: 1,
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(Error::InvalidConfig {
            field: OAuthConfigField::HttpTimeout
        })
    ));
}

struct StalledResolver;

#[async_trait]
impl OAuthDnsResolver for StalledResolver {
    async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>> {
        future::pending().await
    }
}
