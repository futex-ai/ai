//! Per-resource discovery serialization tests.

use std::{sync::atomic::Ordering, time::Duration};

use crate::{Error, McpOAuthDiscovery};

use super::{
    concurrency_support::{
        OTHER_ISSUER, SELECTED_ISSUER, gated_selector, harness, join, other_resource, resource,
        spawn_discovery,
    },
    support::challenge,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_cacheable_discovery_prompts_and_fetches_once() {
    let (selector, gate) = gated_selector();
    let harness = harness(selector, "max-age=60", false);
    let first = spawn_discovery(harness.discovery.clone(), resource());
    assert_eq!(
        gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
        0
    );
    let second = spawn_discovery(harness.discovery.clone(), resource());

    let second_prompted = gate
        .started
        .recv_timeout(Duration::from_millis(250))
        .is_ok();
    gate.release.send(()).unwrap();
    let first_result = join(first).await.unwrap();
    let second_result = join(second).await.unwrap();

    assert!(!second_prompted);
    assert_eq!(first_result, second_result);
    assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.protected_fetches.load(Ordering::SeqCst), 1);
    assert_eq!(harness.server_fetches.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unrelated_resource_discovery_remains_independent() {
    let (selector, gate) = gated_selector();
    let harness = harness(selector, "max-age=60", false);
    let first = spawn_discovery(harness.discovery.clone(), resource());
    assert_eq!(
        gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
        0
    );

    let unrelated = tokio::time::timeout(
        Duration::from_millis(250),
        harness
            .discovery
            .discover(&other_resource(), &challenge(None)),
    )
    .await
    .unwrap()
    .unwrap();

    gate.release.send(()).unwrap();
    assert_eq!(
        unrelated.authorization_server.issuer,
        OTHER_ISSUER.to_owned()
    );
    join(first).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_store_discovery_serializes_without_sharing_results() {
    let (selector, gate) = gated_selector();
    let harness = harness(selector, "no-store", false);
    let first = spawn_discovery(harness.discovery.clone(), resource());
    assert_eq!(
        gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
        0
    );
    let second = spawn_discovery(harness.discovery.clone(), resource());

    let second_prompted_early = gate
        .started
        .recv_timeout(Duration::from_millis(250))
        .is_ok();
    gate.release.send(()).unwrap();
    join(first).await.unwrap();
    if !second_prompted_early {
        assert_eq!(
            gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
            1
        );
    }
    join(second).await.unwrap();

    assert!(!second_prompted_early);
    assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.protected_fetches.load(Ordering::SeqCst), 2);
    assert_eq!(harness.server_fetches.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_discovery_is_not_shared_with_a_waiter() {
    let (selector, gate) = gated_selector();
    let harness = harness(selector, "max-age=60", true);
    let first = spawn_discovery(harness.discovery.clone(), resource());
    assert_eq!(
        gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
        0
    );
    let second = spawn_discovery(harness.discovery.clone(), resource());

    let second_prompted_early = gate
        .started
        .recv_timeout(Duration::from_millis(250))
        .is_ok();
    gate.release.send(()).unwrap();
    let first_error = join(first).await.unwrap_err();
    if !second_prompted_early {
        assert_eq!(
            gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
            1
        );
    }
    let second_result = join(second).await.unwrap();

    assert!(!second_prompted_early);
    assert!(matches!(first_error, Error::Transport));
    assert_eq!(second_result.authorization_server.issuer, SELECTED_ISSUER);
    assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.protected_fetches.load(Ordering::SeqCst), 2);
    assert_eq!(harness.server_fetches.load(Ordering::SeqCst), 2);
}
