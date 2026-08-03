//! Discovery lock cancellation tests.

use std::{sync::atomic::Ordering, time::Duration};

use tokio::sync::oneshot;

use crate::McpOAuthDiscovery;

use super::{
    concurrency_support::{gated_selector, harness, join, resource, spawn_discovery},
    support::challenge,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_a_queued_waiter_does_not_block_later_discovery() {
    let (selector, gate) = gated_selector();
    let harness = harness(selector, "no-store", false);
    let leader = spawn_discovery(harness.discovery.clone(), resource());
    assert_eq!(
        gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
        0
    );

    let (started_tx, started_rx) = oneshot::channel();
    let waiter_discovery = harness.discovery.clone();
    let waiter = tokio::spawn(async move {
        started_tx.send(()).unwrap();
        waiter_discovery
            .discover(&resource(), &challenge(None))
            .await
    });
    started_rx.await.unwrap();
    tokio::task::yield_now().await;
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());

    gate.release.send(()).unwrap();
    join(leader).await.unwrap();
    let later = spawn_discovery(harness.discovery.clone(), resource());
    assert_eq!(
        gate.started.recv_timeout(Duration::from_secs(1)).unwrap(),
        1
    );
    join(later).await.unwrap();

    assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.protected_fetches.load(Ordering::SeqCst), 2);
    assert_eq!(harness.server_fetches.load(Ordering::SeqCst), 2);
}
