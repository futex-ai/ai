//! Authorization endpoint validation-order tests.

use ai_mcp::McpAuthorizationFailure;
use unimock::{MockFn, Unimock, matching};

use crate::{Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager, OAuthEndpointKind};

use super::support::{challenge, context, discovery_result, manager};

#[tokio::test]
async fn rejects_reserved_authorization_parameters_before_registration() {
    for parameter in [
        "response_type",
        "client_id",
        "redirect_uri",
        "code_challenge",
        "code_challenge_method",
        "resource",
        "state",
        "scope",
        "%63lient_id",
    ] {
        let mut discovered = discovery_result();
        discovered.authorization_server.authorization_endpoint =
            format!("https://auth.example/authorize?{parameter}=attacker");
        let oauth = manager(
            Unimock::new(
                McpOAuthDiscoveryMock::discover
                    .next_call(matching!(_, _))
                    .returns(Ok(discovered)),
            ),
            Unimock::new(()),
            Unimock::new(()),
            Unimock::new(()),
            Unimock::new(()),
            Unimock::new(()),
            Unimock::new(()),
            Unimock::new(()),
            McpOAuthConfig::default(),
        );

        let error = oauth
            .authorize(
                &challenge(McpAuthorizationFailure::AuthorizationRequired, &[]),
                &context(),
            )
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidUrl {
                endpoint: OAuthEndpointKind::Authorization
            }
        ));
    }
}
