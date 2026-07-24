//! Shared OAuth discovery fixtures.

use std::{collections::BTreeMap, sync::Arc};

use ai_mcp::{McpAuthorizationChallenge, McpAuthorizationFailure};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{
    AuthorizationServerSelector, CanonicalMcpResource, DefaultMcpOAuthDiscovery, McpOAuthConfig,
    OAuthClock, OAuthClockMock, OAuthHttpResponse, OAuthHttpTransport, OAuthHttpTransportMock,
    OAuthUrlPolicy,
};

pub(super) fn discovery(
    responses: Vec<OAuthHttpResponse>,
    selector: Unimock,
    times: Vec<u64>,
) -> DefaultMcpOAuthDiscovery {
    let responses = Arc::new(std::sync::Mutex::new(
        responses
            .into_iter()
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let transport = Arc::new(Unimock::new(
        OAuthHttpTransportMock::get_json
            .each_call(matching!(_, _, _, _))
            .answers_arc({
                let responses = responses.clone();
                Arc::new(move |_, _, _, _, _| {
                    Ok(responses
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected discovery request"))
                })
            }),
    )) as Arc<dyn OAuthHttpTransport>;
    let times = Arc::new(std::sync::Mutex::new(
        times.into_iter().collect::<std::collections::VecDeque<_>>(),
    ));
    let clock = Arc::new(Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers_arc({
                let times = times.clone();
                Arc::new(move |_| Ok(times.lock().unwrap().pop_front().unwrap()))
            }),
    )) as Arc<dyn OAuthClock>;
    DefaultMcpOAuthDiscovery::new(
        transport,
        Arc::new(selector) as Arc<dyn AuthorizationServerSelector>,
        clock,
        McpOAuthConfig::default(),
    )
    .unwrap()
}

pub(super) fn resource() -> CanonicalMcpResource {
    CanonicalMcpResource::parse("https://mcp.example/api", &OAuthUrlPolicy::default()).unwrap()
}

pub(super) fn challenge(metadata_url: Option<&str>) -> McpAuthorizationChallenge {
    McpAuthorizationChallenge {
        failure: McpAuthorizationFailure::AuthorizationRequired,
        resource_metadata_url: metadata_url.map(str::to_owned),
        scopes: Vec::new(),
        error_description: None,
        raw_www_authenticate: Vec::new(),
    }
}

pub(super) fn protected_json() -> Value {
    json!({
        "resource": "https://mcp.example/api",
        "authorization_servers": ["https://auth.example"]
    })
}

pub(super) fn server_json(issuer: &str) -> Value {
    json!({
        "issuer": issuer,
        "authorization_endpoint": "https://auth.example/authorize",
        "token_endpoint": "https://auth.example/token",
        "token_endpoint_auth_methods_supported": ["none"],
        "code_challenge_methods_supported": ["S256"]
    })
}

pub(super) fn response(body: Value, max_age: u64) -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 200,
        headers: BTreeMap::from([
            (
                "content-type".to_owned(),
                vec!["application/json; charset=utf-8".to_owned()],
            ),
            (
                "cache-control".to_owned(),
                vec![format!("max-age={max_age}")],
            ),
        ]),
        body,
    }
}
