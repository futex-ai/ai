//! Shared support for discovery concurrency tests.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    time::Duration,
};

use serde_json::json;
use tokio::task::JoinHandle;
use unimock::{MockFn, Unimock, matching};

use crate::{
    AuthorizationServerSelector, AuthorizationServerSelectorMock, CanonicalMcpResource,
    DefaultMcpOAuthDiscovery, Error, McpOAuthConfig, McpOAuthDiscovery, OAuthClock, OAuthClockMock,
    OAuthDiscoveryResult, OAuthEndpointKind, OAuthHttpTransport, OAuthHttpTransportMock,
    OAuthUrlPolicy,
};

use super::support::{challenge, response_with_cache_control, server_json};

pub(super) const SELECTED_ISSUER: &str = "https://auth-two.example";
pub(super) const OTHER_ISSUER: &str = "https://auth-other.example";

pub(super) struct SelectorGate {
    pub(super) started: Receiver<usize>,
    pub(super) release: Sender<()>,
    pub(super) calls: Arc<AtomicUsize>,
}

pub(super) fn gated_selector() -> (Unimock, SelectorGate) {
    let calls = Arc::new(AtomicUsize::new(0));
    let (started_tx, started) = channel();
    let (release, release_rx) = channel();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let selector = Unimock::new(
        AuthorizationServerSelectorMock::select
            .each_call(matching!(_, _))
            .answers_arc({
                let calls = calls.clone();
                Arc::new(move |_, _, _| {
                    let call = calls.fetch_add(1, Ordering::SeqCst);
                    started_tx.send(call).unwrap();
                    if call == 0 {
                        release_rx.lock().unwrap().recv().unwrap();
                    }
                    Ok(SELECTED_ISSUER.to_owned())
                })
            }),
    );
    (
        selector,
        SelectorGate {
            started,
            release,
            calls,
        },
    )
}

pub(super) struct DiscoveryHarness {
    pub(super) discovery: Arc<DefaultMcpOAuthDiscovery>,
    pub(super) protected_fetches: Arc<AtomicUsize>,
    pub(super) server_fetches: Arc<AtomicUsize>,
}

pub(super) fn harness(
    selector: Unimock,
    cache_control: &'static str,
    fail_first_server: bool,
) -> DiscoveryHarness {
    let protected_fetches = Arc::new(AtomicUsize::new(0));
    let server_fetches = Arc::new(AtomicUsize::new(0));
    let transport = Arc::new(Unimock::new(
        OAuthHttpTransportMock::get_json
            .each_call(matching!(_, _, _, _))
            .answers_arc({
                let protected_fetches = protected_fetches.clone();
                let server_fetches = server_fetches.clone();
                Arc::new(move |_, url: &str, endpoint, _, _| match endpoint {
                    OAuthEndpointKind::ProtectedResourceMetadata => {
                        protected_fetches.fetch_add(1, Ordering::SeqCst);
                        Ok(protected_response(url, cache_control))
                    }
                    OAuthEndpointKind::AuthorizationServerMetadata => {
                        let fetch = server_fetches.fetch_add(1, Ordering::SeqCst);
                        if fail_first_server && fetch == 0 {
                            Err(Error::Transport)
                        } else {
                            Ok(response_with_cache_control(
                                server_json(issuer_from_metadata_url(url)),
                                cache_control,
                            ))
                        }
                    }
                    _ => unreachable!(),
                })
            }),
    )) as Arc<dyn OAuthHttpTransport>;
    let clock = Arc::new(Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers(&|_| Ok(100)),
    )) as Arc<dyn OAuthClock>;
    let discovery = DefaultMcpOAuthDiscovery::new(
        transport,
        Arc::new(selector) as Arc<dyn AuthorizationServerSelector>,
        clock,
        McpOAuthConfig::default(),
    )
    .unwrap();
    DiscoveryHarness {
        discovery: Arc::new(discovery),
        protected_fetches,
        server_fetches,
    }
}

pub(super) fn resource() -> CanonicalMcpResource {
    CanonicalMcpResource::parse("https://mcp.example/api", &OAuthUrlPolicy::default()).unwrap()
}

pub(super) fn other_resource() -> CanonicalMcpResource {
    CanonicalMcpResource::parse("https://other-mcp.example/api", &OAuthUrlPolicy::default())
        .unwrap()
}

pub(super) fn spawn_discovery(
    discovery: Arc<DefaultMcpOAuthDiscovery>,
    resource: CanonicalMcpResource,
) -> JoinHandle<crate::Result<OAuthDiscoveryResult>> {
    tokio::spawn(async move { discovery.discover(&resource, &challenge(None)).await })
}

pub(super) async fn join(
    task: JoinHandle<crate::Result<OAuthDiscoveryResult>>,
) -> crate::Result<OAuthDiscoveryResult> {
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
}

fn protected_response(url: &str, cache_control: &str) -> crate::OAuthHttpResponse {
    let (resource, issuers) = if url.contains("other-mcp.example") {
        (other_resource().as_str().to_owned(), vec![OTHER_ISSUER])
    } else {
        (
            resource().as_str().to_owned(),
            vec!["https://auth-one.example", SELECTED_ISSUER],
        )
    };
    response_with_cache_control(
        json!({"resource": resource, "authorization_servers": issuers}),
        cache_control,
    )
}

fn issuer_from_metadata_url(url: &str) -> &'static str {
    if url.contains("auth-other.example") {
        OTHER_ISSUER
    } else {
        SELECTED_ISSUER
    }
}
