use crate::behaviour::P2PGoBehaviour;
use anyhow::{Context, Result};
use libp2p::{Multiaddr, PeerId, Swarm};
use std::time::Duration;
use tracing::{info, warn};

/// Bootstrap configuration
pub struct BootstrapConfig {
    /// Known bootstrap nodes (community-run)
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    /// Enable relay client for NAT traversal
    pub enable_relay: bool,
    /// Connection timeout
    pub timeout: Duration,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: vec![
                // Add community bootstrap nodes here
                // For MVP, we'll use mDNS and direct IP connections
            ],
            enable_mdns: true,
            enable_relay: true,
            timeout: Duration::from_secs(30),
        }
    }
}

pub struct Bootstrap {
    config: BootstrapConfig,
}

impl Bootstrap {
    pub fn new(config: BootstrapConfig) -> Self {
        Self { config }
    }

    /// Execute bootstrap process
    pub async fn bootstrap(&self, swarm: &mut Swarm<P2PGoBehaviour>) -> Result<BootstrapResult> {
        info!("Starting bootstrap process...");

        let discovered_peers = Vec::new();
        let mut relay_server = None;

        // Step 1: Enable mDNS for local discovery
        if self.config.enable_mdns {
            info!("mDNS enabled for local peer discovery");
        }

        // Step 2: Connect to bootstrap peers
        for addr in &self.config.bootstrap_peers {
            match swarm.dial(addr.clone()) {
                Ok(_) => {
                    info!("Dialing bootstrap peer: {}", addr);
                }
                Err(e) => {
                    warn!("Failed to dial bootstrap peer {}: {}", addr, e);
                }
            }
        }

        // Step 3: Setup relay if behind NAT
        if self.config.enable_relay {
            // Try to find a relay server
            // For MVP, we'll attempt to use any discovered peer as relay
            tokio::time::sleep(Duration::from_secs(2)).await; // Wait for connections

            let connected_peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
            if let Some(&relay_peer) = connected_peers.first() {
                info!("Attempting to use {} as relay", relay_peer);
                relay_server = Some(relay_peer);

                // Listen on relay
                let relay_addr = Multiaddr::empty()
                    .with(libp2p::multiaddr::Protocol::P2p(relay_peer))
                    .with(libp2p::multiaddr::Protocol::P2pCircuit);

                match swarm.listen_on(relay_addr.clone()) {
                    Ok(_) => {
                        info!("Listening on relay circuit: {}", relay_addr);
                    }
                    Err(e) => {
                        warn!("Failed to listen on relay: {}", e);
                    }
                }
            }
        }

        // Step 4: Bootstrap Kademlia DHT
        if swarm.behaviour_mut().kademlia.bootstrap().is_ok() {
            info!("Kademlia DHT bootstrap initiated");
        }

        Ok(BootstrapResult {
            discovered_peers,
            relay_server,
            local_peer_id: *swarm.local_peer_id(),
        })
    }

    /// Quick bootstrap for two players connecting directly
    pub async fn quick_connect(
        &self,
        swarm: &mut Swarm<P2PGoBehaviour>,
        peer_addr: Multiaddr,
    ) -> Result<()> {
        info!("Quick connect to: {}", peer_addr);

        // Direct dial
        swarm
            .dial(peer_addr.clone())
            .context("Failed to dial peer")?;

        // Subscribe to game topics
        let topics = vec!["p2pgo/games/v1", "p2pgo/rna/v1", "p2pgo/lobby/v1"];

        for topic in topics {
            swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&libp2p::gossipsub::IdentTopic::new(topic))
                .context("Failed to subscribe to topic")?;
        }

        Ok(())
    }
}

pub struct BootstrapResult {
    pub discovered_peers: Vec<PeerId>,
    pub relay_server: Option<PeerId>,
    pub local_peer_id: PeerId,
}
