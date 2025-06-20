//! Debug test for simple gossip functionality

#[cfg(feature = "iroh")]
mod tests {
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};
    use anyhow::Result;

    /// Test basic gossip subscription and broadcasting
    #[tokio::test]
    async fn debug_simple_gossip() -> Result<()> {
        println!("=== Starting Gossip Debug Test ===");
        
        // Create two nodes
        let node_a = Arc::new(IrohCtx::new().await?);
        let node_b = Arc::new(IrohCtx::new().await?);
        
        println!("Created two nodes: {} and {}", node_a.node_id(), node_b.node_id());
        
        // Connect the nodes
        let ticket = node_a.ticket().await?;
        node_b.connect_by_ticket(&ticket).await?;
        
        println!("Connected nodes via ticket");
        
        // Wait for connection to establish
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Subscribe to the same topic on both nodes
        let test_topic = "debug-gossip-test";
        
        println!("Subscribing to gossip topic on both nodes...");
        let mut events_a = node_a.subscribe_game_topic(test_topic, 10).await?;
        let mut events_b = node_b.subscribe_game_topic(test_topic, 10).await?;
        
        println!("Both nodes subscribed to topic");
        
        // Wait for subscriptions to be established
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        // Node A broadcasts a test message
        let test_message = b"Hello from Node A";
        println!("Node A broadcasting test message...");
        let topic_id = IrohCtx::game_topic(test_topic);
        node_a.broadcast_to_topic(topic_id, test_message).await?;
        
        println!("Message broadcast, waiting for receipt...");
        
        // Check if Node B receives the message
        let received = timeout(Duration::from_secs(5), events_b.recv()).await;
        
        match received {
            Ok(Some(event)) => {
                println!("Node B received event: {:?}", event);
                
                // Try to extract message content
                use p2pgo_network::gossip_compat::{extract_bytes, is_received_message};
                if is_received_message(&event) {
                    if let Some(content) = extract_bytes(&event) {
                        let message_str = String::from_utf8_lossy(&content);
                        println!("Message content: {}", message_str);
                        
                        if content == test_message {
                            println!("✓ Test passed! Message received correctly.");
                            return Ok(());
                        } else {
                            println!("✗ Message content mismatch");
                            return Err(anyhow::anyhow!("Message content mismatch"));
                        }
                    } else {
                        println!("✗ Could not extract message content");
                        return Err(anyhow::anyhow!("Could not extract message content"));
                    }
                } else {
                    println!("✗ Event is not a received message");
                    return Err(anyhow::anyhow!("Event is not a received message"));
                }
            }
            Ok(None) => {
                println!("✗ Event stream ended");
                return Err(anyhow::anyhow!("Event stream ended"));
            }
            Err(_) => {
                println!("✗ Timeout waiting for message");
                return Err(anyhow::anyhow!("Timeout waiting for message"));
            }
        }
    }
}
