//! Authorization callback, denial, and timeout tests.

use std::{sync::Arc, time::Duration};

use ai_mcp::McpAuthorizationFailure;
use async_trait::async_trait;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager, OAuthAuthorizationError,
    OAuthAuthorizationResponse, OAuthClientRegistryMock, OAuthRandomMock, OAuthUserAgent,
    OAuthUserAgentMock, OAuthUserAuthorizationRequest, Result,
};

use super::support::{
    challenge, clock, context, discovery_result, manager, manager_with_user_agent, registration,
};

#[tokio::test]
async fn rejects_mismatched_callback_state_before_token_exchange() {
    let oauth = authorization_manager_with_response(OAuthAuthorizationResponse::authorized(
        "code",
        Some("wrong-state"),
    ));

    let error = oauth
        .authorize(
            &challenge(McpAuthorizationFailure::AuthorizationRequired, &[]),
            &context(),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::StateMismatch));
}

#[tokio::test]
async fn maps_callback_errors_without_exposing_callback_values() {
    let oauth = authorization_manager_with_response(OAuthAuthorizationResponse::OAuthError {
        error: OAuthAuthorizationError::TemporarilyUnavailable,
    });

    let error = oauth
        .authorize(
            &challenge(McpAuthorizationFailure::AuthorizationRequired, &[]),
            &context(),
        )
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::AuthorizationRejected {
            error: OAuthAuthorizationError::TemporarilyUnavailable
        }
    ));
}

#[tokio::test]
async fn maps_host_cancellation_without_token_exchange() {
    let oauth = authorization_manager_with_response(OAuthAuthorizationResponse::Cancelled);

    let error = oauth
        .authorize(
            &challenge(McpAuthorizationFailure::AuthorizationRequired, &[]),
            &context(),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::UserCancelled));
}

#[tokio::test]
async fn denied_incremental_scope_is_suppressed_for_the_same_attempt() {
    let user_agent = Unimock::new(
        OAuthUserAgentMock::authorize
            .next_call(matching!(_))
            .returns(Ok(OAuthAuthorizationResponse::OAuthError {
                error: OAuthAuthorizationError::AccessDenied,
            })),
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
        clock(vec![100]),
        random(),
        McpOAuthConfig::default(),
    );
    let scope_challenge = challenge(McpAuthorizationFailure::InsufficientScope, &["write"]);

    let first = oauth.authorize(&scope_challenge, &context()).await;
    let second = oauth.authorize(&scope_challenge, &context()).await;

    assert!(matches!(first, Err(Error::UserDenied)));
    assert!(matches!(second, Err(Error::UserDenied)));
}

#[tokio::test]
async fn enforces_the_user_agent_timeout() {
    let config = McpOAuthConfig {
        user_agent_timeout: Duration::from_millis(5),
        ..McpOAuthConfig::default()
    };
    let oauth = manager_with_user_agent(
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
        Arc::new(PendingUserAgent),
        Unimock::new(()),
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

    assert!(matches!(error, Error::CallbackTimeout));
}

fn authorization_manager_with_response(
    response: OAuthAuthorizationResponse,
) -> crate::DefaultMcpOAuthManager {
    manager(
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
        Unimock::new(
            OAuthUserAgentMock::authorize
                .next_call(matching!(_))
                .returns(Ok(response)),
        ),
        Unimock::new(()),
        clock(vec![100, 101]),
        random(),
        McpOAuthConfig::default(),
    )
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

struct PendingUserAgent;

#[async_trait]
impl OAuthUserAgent for PendingUserAgent {
    async fn authorize(
        &self,
        _request: OAuthUserAuthorizationRequest,
    ) -> Result<OAuthAuthorizationResponse> {
        std::future::pending().await
    }
}
