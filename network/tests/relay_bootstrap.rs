// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for the relay bootstrapping and self-relay features

#[cfg(feature = "iroh")]
mod tests {
    use std::time::Duration;
    use p2pgo_network::config::{NetworkConfig, RelayModeConfig};
    use p2pgo_network::port::PortManager;
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use tokio::time::timeout;
    use anyhow::Result;
    
    // Tests running a self-relay and connecting to it
    #[tokio::test]
    async fn test_self_relay_bootstrap() -> Result<()> {
        // Skip this test on CI if environment indicates
        if std::env::var("CI").is_ok() {
            println!("Skipping relay test on CI");
            return Ok(());
        }

        // Create a port manager for dynamic port selection
        let port_manager = PortManager::new()?;
        let relay_port = port_manager.get_relay_port()?;
        
        // Create a self-relay config
        let self_relay_config = NetworkConfig {
            relay_mode: RelayModeConfig::SelfRelay,
            relay_addrs: vec![],
            gossip_buffer_size: 256,
        };
        
        // Start an Iroh context with self relay
        let iroh_ctx = IrohCtx::new_with_port(self_relay_config, Some(relay_port)).await?;
        
        // Get the node ID and build a custom relay address for the client
        let self_node_id = iroh_ctx.get_node_id();
        println!("Self relay node ID: {}", self_node_id);
        
        // Create a relay address for the local relay
        let relay_addr = format!("/ip4/127.0.0.1/tcp/{}/quic-v1/p2p/{}", relay_port, self_node_id);
        println!("Using relay address: {}", relay_addr);
        
        // Create a client config that connects to the local relay
        let client_config = NetworkConfig {
            relay_mode: RelayModeConfig::Custom,
            relay_addrs: vec![relay_addr],
            gossip_buffer_size: 256,
        };
        
        // Try to create a client context that connects to the local relay
        let client_ctx = timeout(
            Duration::from_secs(5),
            IrohCtx::new_with_config(client_config)
        ).await??;
        
        // Get the client node ID to verify it's different
        let client_node_id = client_ctx.get_node_id();
        println!("Client node ID: {}", client_node_id);
        
        // Verify the client and relay have different node IDs
        assert_ne!(self_node_id, client_node_id, "Client and relay should have different node IDs");
        
        // Test relay connectivity
        let relay_connectivity = client_ctx.test_relay_connectivity().await?;
        println!("Relay connectivity: {:?}", relay_connectivity);
        
        // Check that the relay is online and working
        assert!(relay_connectivity.is_online, "Local relay should be reachable");
        
        Ok(())
    }
    
    // Test relay restart capability
    #[tokio::test]
    async fn test_relay_restart() -> Result<()> {
        // Skip this test on CI if environment indicates
        if std::env::var("CI").is_ok() {
            println!("Skipping relay restart test on CI");
            return Ok(());
        }
        
        // Create a port manager for dynamic port selection
        let port_manager = PortManager::new()?;
        let relay_port = port_manager.get_relay_port()?;
        
        // Create a self-relay config
        let self_relay_config = NetworkConfig {
            relay_mode: RelayModeConfig::SelfRelay,
            relay_addrs: vec![],
            gossip_buffer_size: 256,
        };
        
        // Start an Iroh context with self relay
        let iroh_ctx = IrohCtx::new_with_port(self_relay_config, Some(relay_port)).await?;
        let node_id = iroh_ctx.get_node_id();
        println!("Relay node ID: {}", node_id);
        
        // Initiate a relay shutdown and restart
        println!("Starting relay restart test...");
        iroh_ctx.restart_relay().await?;
        println!("Relay restarted");
        
        // Wait a moment for relay to fully restart
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Verify the node ID is the same after restart
        let new_node_id = iroh_ctx.get_node_id();
        assert_eq!(node_id, new_node_id, "Node ID should remain the same after restart");
        
        // Test relay connectivity after restart
        let relay_connectivity = iroh_ctx.test_relay_connectivity().await?;
        println!("Relay connectivity after restart: {:?}", relay_connectivity);
        
        // Check that the relay is online after restart
        assert!(relay_connectivity.is_online, "Local relay should be reachable after restart");
        
        Ok(())
    }
}

// Empty module for when iroh feature is not enabled
#[cfg(not(feature = "iroh"))]
mod tests {
    #[test]
    fn dummy_test() {
        // Empty test to keep the file valid
        assert!(true);
    }
}
