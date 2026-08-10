//! Authorization-server grant-type validation tests.

use ai_mcp::McpAuthorizationFailure;
use unimock::{MockFn, Unimock, matching};

use crate::{Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager};

use super::support::{challenge, context, discovery_result, manager, server_metadata};

#[test]
fn empty_grant_metadata_uses_authorization_code_default() {
    let mut server = server_metadata();
    server.grant_types_supported.clear();

    assert!(server.supports_authorization_code());

    server.grant_types_supported = vec!["client_credentials".to_owned()];

    assert!(!server.supports_authorization_code());
}

#[tokio::test]
async fn rejects_incompatible_grants_before_side_effects() {
    let mut discovered = discovery_result();
    discovered.authorization_server.grant_types_supported = vec!["client_credentials".to_owned()];
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

    assert!(matches!(error, Error::AuthorizationCodeGrantUnsupported));
}
