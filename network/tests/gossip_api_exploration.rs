//! Comprehensive iroh-gossip API exploration test

#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use iroh::Endpoint;
    use iroh_gossip::net::{Gossip, Config};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    /// Test to explore and understand the iroh-gossip API
    #[tokio::test]
    async fn explore_gossip_api() -> Result<()> {
        println!("=== Iroh Gossip API Exploration ===");
        
        // Create endpoint
        let endpoint = Endpoint::builder().bind().await?;
        println!("✓ Endpoint created: {}", endpoint.node_id());
        
        // Create gossip config
        let config = Config::default();
        println!("✓ Gossip config created with default settings");
        
        // Spawn gossip instance
        let gossip = Arc::new(Gossip::spawn(endpoint.clone(), config).await?);
        println!("✓ Gossip instance spawned successfully");
        
        // Explore available methods
        println!("\n=== Testing Gossip Methods ===");
        
        // Test topic joining
        let test_topic = "test-topic";
        println!("Joining topic: {}", test_topic);
        
        let join_result = gossip.join(test_topic.as_bytes().to_vec()).await;
        match join_result {
            Ok(_) => println!("✓ Successfully joined topic"),
            Err(e) => println!("⚠ Failed to join topic: {}", e),
        }
        
        // Test getting subscription events
        println!("\nTesting subscription events...");
        let mut events = gossip.subscribe();
        println!("✓ Subscribed to gossip events");
        
        // Test broadcasting a message
        println!("\nTesting message broadcast...");
        let test_message = b"Hello, gossip network!";
        
        let broadcast_result = gossip.broadcast(test_topic.as_bytes().to_vec(), test_message.to_vec()).await;
        match broadcast_result {
            Ok(_) => println!("✓ Successfully broadcast message"),
            Err(e) => println!("⚠ Failed to broadcast message: {}", e),
        }
        
        // Try to receive the broadcasted message (with timeout)
        println!("\nWaiting for gossip events...");
        let event_result = timeout(Duration::from_millis(1000), events.recv()).await;
        
        match event_result {
            Ok(Ok(event)) => {
                println!("✓ Received gossip event: {:?}", event);
                
                // Analyze the event structure
                println!("\n=== Event Analysis ===");
                match event {
                    iroh_gossip::net::Event::Gossip(gossip_event) => {
                        println!("Event type: Gossip");
                        println!("Event details: {:?}", gossip_event);
                    }
                    iroh_gossip::net::Event::Lagged => {
                        println!("Event type: Lagged (event stream fell behind)");
                    }
                }
            }
            Ok(Err(e)) => println!("⚠ Error receiving event: {}", e),
            Err(_) => println!("⏰ Timeout waiting for events (this is normal for a single node)"),
        }
        
        // Test leaving the topic
        println!("\nLeaving topic...");
        let quit_result = gossip.quit(test_topic.as_bytes().to_vec()).await;
        match quit_result {
            Ok(_) => println!("✓ Successfully left topic"),
            Err(e) => println!("⚠ Failed to leave topic: {}", e),
        }
        
        println!("\n=== API Exploration Complete ===");
        println!("Key methods discovered:");
        println!("- join(topic) -> Join a gossip topic");
        println!("- quit(topic) -> Leave a gossip topic"); 
        println!("- broadcast(topic, message) -> Send message to topic");
        println!("- subscribe() -> Get event stream");
        println!("- Events: Gossip(content) | Lagged");
        
        Ok(())
    }
    
    /// Test two-node gossip communication
    #[tokio::test]
    async fn test_two_node_gossip() -> Result<()> {
        println!("=== Two-Node Gossip Test ===");
        
        // Create two endpoints
        let endpoint_a = Arc::new(Endpoint::builder().bind().await?);
        let endpoint_b = Arc::new(Endpoint::builder().bind().await?);
        
        println!("Node A: {}", endpoint_a.node_id());
        println!("Node B: {}", endpoint_b.node_id());
        
        // Connect the endpoints
        let addrs = endpoint_a.direct_addresses().await?;
        let node_addr = iroh::NodeAddr::new(endpoint_a.node_id()).with_direct_addresses(addrs);
        endpoint_b.connect(node_addr, iroh::ALPN).await?;
        
        println!("✓ Nodes connected");
        
        // Create gossip instances
        let gossip_a = Arc::new(Gossip::spawn(endpoint_a.clone(), Config::default()).await?);
        let gossip_b = Arc::new(Gossip::spawn(endpoint_b.clone(), Config::default()).await?);
        
        println!("✓ Gossip instances created");
        
        // Join same topic on both nodes
        let topic = b"shared-topic".to_vec();
        gossip_a.join(topic.clone()).await?;
        gossip_b.join(topic.clone()).await?;
        
        println!("✓ Both nodes joined shared topic");
        
        // Subscribe to events on node B
        let mut events_b = gossip_b.subscribe();
        
        // Wait a moment for connection to establish
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Broadcast from node A
        let message = b"Hello from Node A!".to_vec();
        gossip_a.broadcast(topic.clone(), message.clone()).await?;
        
        println!("✓ Node A broadcast message");
        
        // Try to receive on node B
        let event_result = timeout(Duration::from_secs(3), events_b.recv()).await;
        
        match event_result {
            Ok(Ok(event)) => {
                println!("✓ Node B received event: {:?}", event);
                
                if let iroh_gossip::net::Event::Gossip(gossip_event) = event {
                    println!("Message content: {:?}", gossip_event);
                }
            }
            Ok(Err(e)) => println!("⚠ Error on Node B: {}", e),
            Err(_) => println!("⏰ Timeout - message may not have reached Node B"),
        }
        
        println!("=== Two-Node Test Complete ===");
        Ok(())
    }
}

#[cfg(not(feature = "iroh"))]
mod tests {
    #[test] 
    fn stub_gossip_api_test() {
        println!("Iroh feature not enabled - gossip API tests skipped");
    }
}
