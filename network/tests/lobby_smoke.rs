#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use std::time::Duration;
    use tokio::time::timeout;
    use uuid::Uuid;

    #[tokio::test]
    async fn lobby_gossip_between_nodes() -> Result<()> {
        // Create two nodes
        let node_a = IrohCtx::new().await?;
        let node_b = IrohCtx::new().await?;
        
        // Wait for nodes to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Use a fixed board size for testing
        let board_size = 9;
        
        // Subscribe to the same topic on both nodes
        let mut events_a = node_a.subscribe_lobby(board_size).await?;
        let mut events_b = node_b.subscribe_lobby(board_size).await?;
        
        // Wait for subscriptions to be ready
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Generate a unique game ID
        let game_id = Uuid::new_v4().to_string();
        
        // Node A broadcasts a game advertisement
        node_a.advertise_game(&game_id, board_size).await?;
        
        // Node B should receive the advertisement within 2 seconds
        let event_received = timeout(
            Duration::from_secs(2),
            async {
                while let Some(event) = events_b.recv().await {
                    println!("Received gossip event: {:?}", event);
                    // In iroh-gossip v0.35, we need to match on the event type first
                    if let iroh_gossip::net::Event::Gossip(gossip_event) = event {
                        // Use our helper function to get bytes
                        use p2pgo_network::gossip_compat::extract_bytes;
                        let bytes = extract_bytes(&gossip_event);
                        
                        // Try to decode as string or check raw bytes
                        if let Ok(s) = std::str::from_utf8(&bytes) {
                            if s.contains(&game_id) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
        ).await?;
        
        assert!(event_received, "Node B should have received the game advertisement from node A");
        
        Ok(())
    }
}

// When iroh feature is not enabled, this test module doesn't exist
#[cfg(not(feature = "iroh"))]
#[test]
fn stub_builds_successfully() {
    // This test exists just to ensure the file compiles without the iroh feature
    println!("Stub build works correctly");
}
