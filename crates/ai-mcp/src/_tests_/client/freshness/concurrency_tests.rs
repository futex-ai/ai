//! Concurrent tool-list invalidation tests.

use std::sync::Arc;

use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::{Error, McpClient, McpServerConfig, StreamableHttpMcpClient};

use super::support::{CallBehavior, TwoPageRaceTransport};

#[tokio::test]
async fn concurrent_call_invalidation_during_list_remains_stale() {
    let transport = Arc::new(TwoPageRaceTransport::default());
    let client = Arc::new(
        StreamableHttpMcpClient::new(
            transport.clone(),
            Arc::new(StaticHeaderAuth::default()),
            McpServerConfig::new("demo", "https://example.com/mcp"),
        )
        .unwrap(),
    );
    client.ensure_initialized().await.unwrap();
    let listing_client = client.clone();
    let listing = tokio::spawn(async move { listing_client.list_tools().await });
    transport.wait_until_final_page().await;

    client.call_tool("change", json!({})).await.unwrap();
    assert!(client.tools_list_changed());
    transport.release_final_page();
    listing.await.unwrap().unwrap();

    assert!(client.tools_list_changed());
}

#[tokio::test]
async fn older_list_completion_does_not_regress_newer_acknowledgement() {
    let (client, transport) = client_and_transport(CallBehavior::ListChanged);
    let listing_client = client.clone();
    let older_listing = tokio::spawn(async move { listing_client.list_tools().await });
    transport.wait_until_final_page().await;
    client.call_tool("change", json!({})).await.unwrap();

    client.list_tools().await.unwrap();
    assert!(!client.tools_list_changed());
    transport.release_final_page();
    older_listing.await.unwrap().unwrap();

    assert!(!client.tools_list_changed());
}

#[tokio::test]
async fn matching_session_expiry_during_list_remains_stale() {
    let (client, transport) = client_and_transport(CallBehavior::SessionExpired);
    let listing_client = client.clone();
    let listing = tokio::spawn(async move { listing_client.list_tools().await });
    transport.wait_until_final_page().await;

    let error = client.call_tool("expire", json!({})).await.unwrap_err();
    assert!(matches!(error, Error::SessionExpired));
    transport.release_final_page();
    listing.await.unwrap().unwrap();

    assert!(client.tools_list_changed());
}

fn client_and_transport(
    call_behavior: CallBehavior,
) -> (Arc<StreamableHttpMcpClient>, Arc<TwoPageRaceTransport>) {
    let transport = Arc::new(TwoPageRaceTransport::new(call_behavior));
    let client = Arc::new(
        StreamableHttpMcpClient::new(
            transport.clone(),
            Arc::new(StaticHeaderAuth::default()),
            McpServerConfig::new("demo", "https://example.com/mcp"),
        )
        .unwrap(),
    );
    (client, transport)
}
