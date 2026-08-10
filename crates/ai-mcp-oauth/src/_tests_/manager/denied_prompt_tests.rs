//! Incremental-consent denial identity and lifecycle tests.

use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
};

use ai_mcp::McpAuthorizationFailure;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};
use url::Url;

use crate::{
    DefaultMcpOAuthManager, Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager,
    OAuthAuthorizationError, OAuthAuthorizationResponse, OAuthClientRegistryMock, OAuthClockMock,
    OAuthCredentialStoreMock, OAuthDnsResolverMock, OAuthHttpResponse, OAuthHttpTransportMock,
    OAuthRandomMock, OAuthUserAgentMock, OAuthUserAuthorizationRequest,
};

use super::support::{challenge, context, discovery_result, manager, registration};

#[tokio::test]
async fn reordered_scope_denial_is_suppressed_for_one_attempt() {
    let fixture = denial_fixture([Decision::Denied]);
    let first = challenge(
        McpAuthorizationFailure::InsufficientScope,
        &["write", "admin"],
    );
    let reordered = challenge(
        McpAuthorizationFailure::InsufficientScope,
        &["admin", "write"],
    );

    let first_result = fixture.manager.authorize(&first, &context()).await;
    let second_result = fixture.manager.authorize(&reordered, &context()).await;

    assert!(matches!(first_result, Err(Error::UserDenied)));
    assert!(matches!(second_result, Err(Error::UserDenied)));
    assert_eq!(fixture.user_agent_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn distinct_scope_sets_are_prompted_independently() {
    let fixture = denial_fixture([Decision::Denied, Decision::Denied]);
    let write = challenge(McpAuthorizationFailure::InsufficientScope, &["write"]);
    let admin = challenge(McpAuthorizationFailure::InsufficientScope, &["admin"]);

    let write_result = fixture.manager.authorize(&write, &context()).await;
    let admin_result = fixture.manager.authorize(&admin, &context()).await;

    assert!(matches!(write_result, Err(Error::UserDenied)));
    assert!(matches!(admin_result, Err(Error::UserDenied)));
    assert_eq!(fixture.user_agent_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn distinct_attempts_are_prompted_independently() {
    let fixture = denial_fixture([Decision::Denied, Decision::Denied]);
    let scope = challenge(McpAuthorizationFailure::InsufficientScope, &["write"]);
    let first_context = context();
    let mut second_context = context();
    second_context.authorization_attempt_id = "attempt-2".to_owned();

    let first_result = fixture.manager.authorize(&scope, &first_context).await;
    let second_result = fixture.manager.authorize(&scope, &second_context).await;

    assert!(matches!(first_result, Err(Error::UserDenied)));
    assert!(matches!(second_result, Err(Error::UserDenied)));
    assert_eq!(fixture.user_agent_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn successful_authorization_clears_reordered_scope_denial() {
    let fixture = denial_fixture([Decision::Denied, Decision::Authorized, Decision::Denied]);
    let first = challenge(
        McpAuthorizationFailure::InsufficientScope,
        &["write", "admin"],
    );
    let reordered = challenge(McpAuthorizationFailure::InvalidToken, &["admin", "write"]);

    let first_result = fixture.manager.authorize(&first, &context()).await;
    let authorized = fixture.manager.authorize(&reordered, &context()).await;
    let retry_result = fixture.manager.authorize(&first, &context()).await;

    assert!(matches!(first_result, Err(Error::UserDenied)));
    assert!(authorized.is_ok());
    assert!(matches!(retry_result, Err(Error::UserDenied)));
    assert_eq!(fixture.user_agent_calls.load(Ordering::SeqCst), 3);
}

#[derive(Clone, Copy)]
enum Decision {
    Denied,
    Authorized,
}

struct DenialFixture {
    manager: DefaultMcpOAuthManager,
    user_agent_calls: Arc<AtomicUsize>,
}

fn denial_fixture(decisions: impl IntoIterator<Item = Decision>) -> DenialFixture {
    let decisions = decisions.into_iter().collect::<VecDeque<_>>();
    let has_authorized = decisions
        .iter()
        .any(|decision| matches!(decision, Decision::Authorized));
    let decisions = Arc::new(Mutex::new(decisions));
    let user_agent_calls = Arc::new(AtomicUsize::new(0));
    let user_agent = Unimock::new(
        OAuthUserAgentMock::authorize
            .each_call(matching!(_))
            .answers_arc({
                let decisions = decisions.clone();
                let user_agent_calls = user_agent_calls.clone();
                Arc::new(move |_, request| {
                    user_agent_calls.fetch_add(1, Ordering::SeqCst);
                    let decision = decisions
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected user-agent call");
                    let state = callback_state(request);
                    Ok(match decision {
                        Decision::Denied => OAuthAuthorizationResponse::oauth_error(
                            OAuthAuthorizationError::AccessDenied,
                            Some(state),
                        ),
                        Decision::Authorized => {
                            OAuthAuthorizationResponse::authorized("code", Some(state))
                        }
                    })
                })
            }),
    );
    let random_byte = Arc::new(AtomicU8::new(1));
    let store = if has_authorized {
        Unimock::new(
            OAuthCredentialStoreMock::save_tokens
                .each_call(matching!(_, _))
                .answers(&|_, _, _| Ok(())),
        )
    } else {
        Unimock::new(())
    };
    let transport = if has_authorized {
        Unimock::new(
            OAuthHttpTransportMock::post_form
                .each_call(matching!(_, _, _, _, _))
                .answers(&|_, _, _, _, _, _| {
                    Ok(OAuthHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: json!({
                            "access_token": "access",
                            "token_type": "Bearer"
                        }),
                    })
                }),
        )
    } else {
        Unimock::new(())
    };
    let manager = manager(
        Unimock::new(
            McpOAuthDiscoveryMock::discover
                .each_call(matching!(_, _))
                .answers(&|_, _, _| Ok(discovery_result())),
        ),
        Unimock::new(
            OAuthClientRegistryMock::resolve
                .each_call(matching!(_))
                .answers(&|_, _| Ok(registration())),
        ),
        store,
        user_agent,
        transport,
        Unimock::new(
            OAuthDnsResolverMock::resolve
                .each_call(matching!("auth.example", 443))
                .answers(&|_, _, _| Ok(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))])),
        ),
        Unimock::new(
            OAuthClockMock::now_unix_seconds
                .each_call(matching!())
                .answers(&|_| Ok(100)),
        ),
        Unimock::new(
            OAuthRandomMock::bytes
                .each_call(matching!(32))
                .answers_arc(Arc::new(move |_, _| {
                    let value = random_byte.fetch_add(1, Ordering::SeqCst);
                    Ok(vec![value; 32])
                })),
        ),
        McpOAuthConfig::default(),
    );
    DenialFixture {
        manager,
        user_agent_calls,
    }
}

fn callback_state(request: OAuthUserAuthorizationRequest) -> String {
    Url::parse(request.authorization_url())
        .unwrap()
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned()
}
