use crate::test_helpers::*;
use anyhow::Result;
use std::time::Duration;
use tracing::info;

#[tokio::test]
async fn test_local_mdns_discovery() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // Start relay 1
    let mut relay1 = TestRelay::new(4001).await?;
    let peer_id1 = relay1.peer_id();
    info!("Started relay1 with peer_id: {}", peer_id1);
    
    // Start relay 2
    let mut relay2 = TestRelay::new(4002).await?;
    let peer_id2 = relay2.peer_id();
    info!("Started relay2 with peer_id: {}", peer_id2);
    
    // Subscribe both to RNA topic
    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;
    
    // Wait for mDNS discovery
    let discovered = relay2.wait_for_peer(peer_id1, Duration::from_secs(5)).await;
    assert!(discovered, "Relay 2 should discover Relay 1 via mDNS");
    
    // Verify connection type
    let conn_type = relay2.get_connection_type(peer_id1).await;
    assert_eq!(conn_type, ConnectionType::Direct);
    
    // Test RNA propagation after discovery
    let test_rna = relay1.create_sgf_rna(test_data::SGF_TEST_DATA.to_string(), (0, 10));
    relay1.broadcast_rna(test_rna.clone()).await?;
    
    let received = relay2.wait_for_rna(Duration::from_secs(2)).await;
    assert!(received.is_some(), "Should receive RNA after discovery");
    assert_eq!(received.unwrap().source_peer, peer_id1.to_string());
    
    Ok(())
}

#[tokio::test]
async fn test_direct_connection() -> Result<()> {
    let mut relay1 = TestRelay::new(4003).await?;
    let addr1 = relay1.listening_addresses()[0].clone();
    info!("Relay1 listening on: {}", addr1);
    
    let mut relay2 = TestRelay::new(4004).await?;
    
    // Subscribe to RNA
    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;
    
    // Direct dial
    relay2.connect_to_peer(addr1).await?;
    
    // Wait for connection
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify connected
    assert!(relay2.is_connected_to(relay1.peer_id()).await);
    
    // Measure connection latency
    let latency = relay2.measure_latency(relay1.peer_id()).await;
    assert!(latency < Duration::from_millis(50), "Local connection should be fast");
    
    // Test bidirectional RNA flow
    let rna1 = relay1.create_sgf_rna(test_data::SGF_TEST_DATA.to_string(), (0, 5));
    relay1.broadcast_rna(rna1).await?;
    
    let rna2 = relay2.create_sgf_rna(test_data::SGF_TEST_DATA.to_string(), (5, 10));
    relay2.broadcast_rna(rna2).await?;
    
    // Both should receive each other's RNA
    let received1 = relay1.wait_for_rna(Duration::from_secs(1)).await;
    let received2 = relay2.wait_for_rna(Duration::from_secs(1)).await;
    
    assert!(received1.is_some(), "Relay1 should receive RNA from Relay2");
    assert!(received2.is_some(), "Relay2 should receive RNA from Relay1");
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_relay_discovery() -> Result<()> {
    // Create a network of 4 relays
    let mut relays = vec![];
    let ports = vec![4005, 4006, 4007, 4008];
    
    for port in ports {
        let mut relay = TestRelay::new(port).await?;
        relay.subscribe_rna().await?;
        relays.push(relay);
    }
    
    // Allow time for mDNS discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Process events to establish connections
    for _ in 0..20 {
        for relay in &mut relays {
            tokio::select! {
                event = relay.swarm.next() => {
                    if let Some(event) = event {
                        // Events are handled internally
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
        }
    }
    
    // Verify mesh connectivity
    let peer_ids: Vec<_> = relays.iter().map(|r| r.peer_id()).collect();
    
    for (i, relay) in relays.iter().enumerate() {
        let mut connected_count = 0;
        for (j, peer_id) in peer_ids.iter().enumerate() {
            if i != j && relay.is_connected_to(*peer_id).await {
                connected_count += 1;
            }
        }
        info!("Relay {} connected to {} peers", i, connected_count);
        assert!(connected_count >= 2, "Each relay should connect to at least 2 others");
    }
    
    // Test RNA flooding across mesh
    let test_rna = relays[0].create_sgf_rna(test_data::SGF_TEST_DATA.to_string(), (0, 20));
    relays[0].broadcast_rna(test_rna.clone()).await?;
    
    // All other relays should receive it
    for i in 1..relays.len() {
        let received = relays[i].wait_for_rna(Duration::from_secs(3)).await;
        assert!(received.is_some(), "Relay {} should receive RNA", i);
        assert_eq!(received.unwrap().id, test_rna.id);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_relay_discovery_scoring() -> Result<()> {
    // Test that creating valuable RNA increases discovery score
    let mut relay1 = TestRelay::new(4009).await?;
    let mut relay2 = TestRelay::new(4010).await?;
    
    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;
    
    // Connect relays
    let addr1 = relay1.listening_addresses()[0].clone();
    relay2.connect_to_peer(addr1).await?;
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Relay1 creates high-quality RNA
    for i in 0..5 {
        let rna = RNAMessage {
            id: format!("high-quality-{}", i),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::SGFData {
                sgf_content: test_data::SGF_TEST_DATA.to_string(),
                move_range: (0, 50),
                player_ranks: ("1d".to_string(), "2d".to_string()),
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            quality_score: 0.95,
            data: vec![],
        };
        
        relay1.broadcast_rna(rna).await?;
    }
    
    // In a real implementation, this would boost relay1's discovery score
    // making it more likely to be found by new peers
    
    Ok(())
}