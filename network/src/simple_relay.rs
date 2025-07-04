//! Simple 2-relay direct connection for testing
//! 
//! This module provides a simplified networking setup for testing with just 2 relays
//! that connect directly without complex discovery mechanisms.

use anyhow::Result;
use libp2p::{
    identity::Keypair,
    multiaddr::Multiaddr,
    PeerId,
};
use tracing::{info};

/// Simple relay configuration for 2-node testing
pub struct SimpleRelayConfig {
    /// Local port to listen on
    pub port: u16,
    /// Remote relay address (if not the first relay)
    pub remote_relay: Option<Multiaddr>,
    /// Enable relay services
    pub enable_relay: bool,
}

impl Default for SimpleRelayConfig {
    fn default() -> Self {
        Self {
            port: 4001,
            remote_relay: None,
            enable_relay: true,
        }
    }
}

/// Simple 2-relay network for testing
pub struct SimpleRelay {
    #[allow(dead_code)]
    keypair: Keypair,
    #[allow(dead_code)]
    peer_id: PeerId,
    config: SimpleRelayConfig,
}

impl SimpleRelay {
    pub fn new(config: SimpleRelayConfig) -> Result<Self> {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        
        info!("Starting simple relay with peer ID: {}", peer_id);
        
        Ok(Self {
            keypair,
            peer_id,
            config,
        })
    }
    
    /// Connect directly to another relay
    pub async fn connect_to_relay(&mut self, addr: &Multiaddr) -> Result<()> {
        info!("Connecting directly to relay at: {}", addr);
        // Simple direct connection logic here
        Ok(())
    }
    
    /// Start the relay
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting simple relay on port {}", self.config.port);
        
        // If we have a remote relay configured, connect to it
        if let Some(remote) = self.config.remote_relay.clone() {
            self.connect_to_relay(&remote).await?;
        }
        
        Ok(())
    }
}

/// Helper to create relay addresses for testing
pub fn create_relay_addr(host: &str, port: u16) -> Result<Multiaddr> {
    let addr = format!("/ip4/{}/tcp/{}", host, port).parse()?;
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_two_relay_setup() {
        // First relay (bootstrap)
        let relay1_config = SimpleRelayConfig {
            port: 4001,
            remote_relay: None,
            enable_relay: true,
        };
        
        // Second relay connects to first
        let relay2_config = SimpleRelayConfig {
            port: 4002,
            remote_relay: Some(create_relay_addr("127.0.0.1", 4001).unwrap()),
            enable_relay: true,
        };
        
        let mut relay1 = SimpleRelay::new(relay1_config).unwrap();
        let mut relay2 = SimpleRelay::new(relay2_config).unwrap();
        
        relay1.start().await.unwrap();
        relay2.start().await.unwrap();
    }
}