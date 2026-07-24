//! OAuth configuration validation tests.

use std::time::Duration;

use crate::{Error, McpOAuthConfig, OAuthConfigField};

#[test]
fn defaults_match_the_protocol_contract() {
    let config = McpOAuthConfig::default();

    assert_eq!(config.http_timeout, Duration::from_secs(30));
    assert_eq!(config.user_agent_timeout, Duration::from_secs(600));
    assert_eq!(config.state_lifetime, Duration::from_secs(600));
    assert_eq!(config.max_response_bytes, 1024 * 1024);
    assert_eq!(config.max_redirects, 3);
    assert_eq!(config.max_metadata_cache_age, Duration::from_secs(3600));
    assert_eq!(config.refresh_skew, Duration::from_secs(60));
    assert!(config.validate().is_ok());
}

#[test]
fn zero_bounds_fail_before_side_effects() {
    let mut config = McpOAuthConfig {
        http_timeout: Duration::ZERO,
        ..McpOAuthConfig::default()
    };
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfig {
            field: OAuthConfigField::HttpTimeout
        })
    ));

    config.http_timeout = Duration::from_secs(1);
    config.max_response_bytes = 0;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfig {
            field: OAuthConfigField::ResponseLimit
        })
    ));

    config.max_response_bytes = 1;
    config.refresh_skew = Duration::ZERO;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfig {
            field: OAuthConfigField::RefreshSkew
        })
    ));
}
