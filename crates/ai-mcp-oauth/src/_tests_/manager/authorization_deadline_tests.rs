//! Effective host-visible authorization deadline tests.

use std::{sync::Arc, time::Duration};

use ai_mcp::McpAuthorizationFailure;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager, OAuthAuthorizationResponse,
    OAuthClientRegistryMock, OAuthRandomMock, OAuthUserAgentMock, OAuthUserAuthorizationRequest,
};

use super::support::{
    challenge, clock, context, discovery_result, manager, public_dns_resolver, registration,
};

#[tokio::test]
async fn authorization_deadline_uses_shorter_user_agent_timeout() {
    let config = McpOAuthConfig {
        user_agent_timeout: Duration::from_secs(60),
        state_lifetime: Duration::from_secs(600),
        ..McpOAuthConfig::default()
    };

    assert_deadline(config, 160).await;
}

#[tokio::test]
async fn authorization_deadline_uses_shorter_state_lifetime() {
    let config = McpOAuthConfig {
        user_agent_timeout: Duration::from_secs(600),
        state_lifetime: Duration::from_secs(60),
        ..McpOAuthConfig::default()
    };

    assert_deadline(config, 160).await;
}

async fn assert_deadline(config: McpOAuthConfig, expected: u64) {
    let user_agent = Unimock::new(
        OAuthUserAgentMock::authorize
            .next_call(matching!(_))
            .answers_arc(Arc::new(
                move |_, request: OAuthUserAuthorizationRequest| {
                    assert_eq!(request.expires_at(), expected);
                    Ok(OAuthAuthorizationResponse::Cancelled)
                },
            )),
    );
    let oauth = manager(
        Unimock::new(
            McpOAuthDiscoveryMock::discover
                .next_call(matching!(_, _))
                .returns(Ok(discovery_result())),
        ),
        Unimock::new(
            OAuthClientRegistryMock::resolve
                .next_call(matching!(_))
                .returns(Ok(registration())),
        ),
        Unimock::new(()),
        user_agent,
        Unimock::new(()),
        public_dns_resolver(),
        clock(vec![100]),
        random(),
        config,
    );

    let error = oauth
        .authorize(
            &challenge(McpAuthorizationFailure::AuthorizationRequired, &[]),
            &context(),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::UserCancelled));
}

fn random() -> Unimock {
    Unimock::new((
        OAuthRandomMock::bytes
            .next_call(matching!(32))
            .returns(Ok(vec![1; 32])),
        OAuthRandomMock::bytes
            .next_call(matching!(32))
            .returns(Ok(vec![2; 32])),
    ))
}
