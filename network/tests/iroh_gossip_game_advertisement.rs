// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test iroh gossip game advertisement

#[cfg(feature = "iroh")]
mod tests {
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_game_advertisement_gossip() {
        // Create two iroh contexts
        let ctx1 = IrohCtx::new().await.expect("Failed to create first IrohCtx");
        let ctx2 = IrohCtx::new().await.expect("Failed to create second IrohCtx");
        
        // Connect ctx2 to ctx1
        let ticket = ctx1.ticket().await.expect("Failed to generate ticket");
        ctx2.connect_by_ticket(&ticket).await.expect("Failed to connect by ticket");
        
        // Subscribe ctx2 to lobby for board size 9
        let mut rx = ctx2.subscribe_lobby(9).await.expect("Failed to subscribe to lobby");
        
        // Give a moment for subscription to be established
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Advertise a game from ctx1
        ctx1.advertise_game("test-game-123", 9).await.expect("Failed to advertise game");
        
        // Wait for the advertisement to arrive
        let event = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("Timeout waiting for gossip event")
            .expect("Failed to receive gossip event");
            
        // Check if this is a game advertisement
        // In iroh-gossip v0.35, Event::Gossip contains a GossipEvent
        use iroh_gossip::net::Event;
        match event {
            Event::Gossip(gossip_event) => {
                // In iroh-gossip v0.35, use our helper function to get bytes
                use p2pgo_network::gossip_compat::extract_bytes;
                let bytes = extract_bytes(&gossip_event);
                
                // Decode as CBOR
                let ad: p2pgo_network::iroh_endpoint::GameAdvert = 
                    serde_cbor::from_slice(&bytes).expect("Failed to decode game advertisement");
                    
                assert_eq!(ad.gid, "test-game-123");
                assert_eq!(ad.size, 9);
                assert_eq!(ad.host, ctx1.node_id());
                assert_eq!(ad.bot, false);
            }
            Event::Lagged => {
                panic!("Unexpected lagged event");
            }
        }
    }
    
    #[tokio::test]
    async fn test_multi_board_size_lobbies() {
        // Test that different board sizes use separate lobby topics
        let ctx1 = IrohCtx::new().await.expect("Failed to create first IrohCtx");
        let ctx2 = IrohCtx::new().await.expect("Failed to create second IrohCtx");
        
        // Connect contexts
        let ticket = ctx1.ticket().await.expect("Failed to generate ticket");
        ctx2.connect_by_ticket(&ticket).await.expect("Failed to connect by ticket");
        
        // Subscribe to different board sizes
        let mut rx_9 = ctx2.subscribe_lobby(9).await.expect("Failed to subscribe to 9x9 lobby");
        let mut rx_19 = ctx2.subscribe_lobby(19).await.expect("Failed to subscribe to 19x19 lobby");
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Advertise 19x19 game
        ctx1.advertise_game("test-game-19x19", 19).await.expect("Failed to advertise 19x19 game");
        
        // rx_19 should receive the advertisement
        let event_19 = timeout(Duration::from_secs(2), rx_19.recv())
            .await
            .expect("Timeout waiting for 19x19 gossip event")
            .expect("Failed to receive 19x19 gossip event");
            
        // rx_9 should NOT receive it (different topic)
        let result_9 = timeout(Duration::from_millis(500), rx_9.recv()).await;
        assert!(result_9.is_err(), "9x9 lobby should not receive 19x19 advertisement");
        
        // Verify the 19x19 advertisement content
        use iroh_gossip::net::Event;
        match event_19 {
            Event::Gossip(gossip_event) => {
                // Use our helper function to get bytes
                use p2pgo_network::gossip_compat::extract_bytes;
                let content = extract_bytes(&gossip_event);
                let ad: p2pgo_network::iroh_endpoint::GameAdvert = 
                    serde_cbor::from_slice(&content).expect("Failed to decode game advertisement");
                assert_eq!(ad.size, 19);
            }
            Event::Lagged => {
                panic!("Expected Gossip event, got Lagged");
            }
        }
    }
}

#[cfg(not(feature = "iroh"))]
mod stub_tests {
    use p2pgo_network::iroh_endpoint::IrohCtx;
    
    #[tokio::test]
    async fn test_stub_game_advertisement() {
        let ctx = IrohCtx::new().await.expect("Failed to create stub IrohCtx");
        
        // Should succeed in stub mode but do nothing
        let result = ctx.advertise_game("test-game", 9).await;
        assert!(result.is_ok(), "Stub game advertisement should succeed");
        
        // Subscribe should also work
        let result = ctx.subscribe_lobby(9).await;
        assert!(result.is_ok(), "Stub lobby subscription should succeed");
    }
}
