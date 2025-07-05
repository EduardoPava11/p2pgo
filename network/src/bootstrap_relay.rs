use anyhow::Result;
use futures::StreamExt;
use libp2p::{
    core::upgrade,
    gossipsub, identity,
    kad::{self, store::MemoryStore},
    mdns, noise, relay,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder, Transport,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

use crate::rna::{RNAMessage, RNAType};

/// Bootstrap relay node - The first relay in the network
/// This relay:
/// 1. Makes itself discoverable via mDNS and DHT
/// 2. Acts as a circuit relay for NAT traversal
/// 3. Maintains network health metrics
/// 4. Broadcasts its presence periodically
pub struct BootstrapRelay {
    swarm: Swarm<BootstrapBehaviour>,
    peer_id: PeerId,
    /// Connected peers and their info
    connected_peers: HashMap<PeerId, PeerInfo>,
    /// Network metrics
    metrics: NetworkMetrics,
    /// Discovery boost from creating valuable RNA
    discovery_score: f32,
}

#[derive(Clone)]
struct PeerInfo {
    connected_at: std::time::Instant,
    last_seen: std::time::Instant,
    data_sent_kb: f32,
    data_received_kb: f32,
    is_relay_client: bool,
}

#[allow(dead_code)]
struct NetworkMetrics {
    total_peers_connected: usize,
    active_connections: usize,
    rna_messages_propagated: usize,
    total_bandwidth_kb: f32,
    uptime: std::time::Instant,
}

#[derive(NetworkBehaviour)]
pub struct BootstrapBehaviour {
    pub relay: relay::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub identify: libp2p::identify::Behaviour,
}

impl BootstrapRelay {
    /// Create a new bootstrap relay
    pub async fn new(port: u16) -> Result<Self> {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        info!("Starting bootstrap relay with peer_id: {}", peer_id);

        // Create transport
        let _transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&keypair)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // Configure relay
        let relay_cfg = relay::Config::default();
        let relay_behaviour = relay::Behaviour::new(peer_id, relay_cfg);

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(5))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid gossipsub config: {:?}", e))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create gossipsub: {}", e))?;

        // Configure Kademlia DHT
        let store = MemoryStore::new(peer_id);
        let mut kademlia = kad::Behaviour::with_config(peer_id, store, kad::Config::default());

        // Bootstrap Kademlia
        kademlia.bootstrap()?;

        // Configure mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;

        // Configure identify
        let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
            "/p2pgo/1.0.0".to_string(),
            keypair.public(),
        ));

        let behaviour = BootstrapBehaviour {
            relay: relay_behaviour,
            gossipsub,
            kademlia,
            mdns,
            identify,
        };

        let mut swarm = SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_| Ok(behaviour))?
            .build();

        // Listen on all interfaces
        swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", port).parse()?)?;

        // Also listen on IPv6
        swarm.listen_on(format!("/ip6/::/tcp/{}", port).parse()?)?;

        Ok(Self {
            swarm,
            peer_id,
            connected_peers: HashMap::new(),
            metrics: NetworkMetrics {
                total_peers_connected: 0,
                active_connections: 0,
                rna_messages_propagated: 0,
                total_bandwidth_kb: 0.0,
                uptime: std::time::Instant::now(),
            },
            discovery_score: 1.0, // Bootstrap starts with high discovery score
        })
    }

    /// Get the peer ID of this relay
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Get listening addresses
    pub fn listening_addresses(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Subscribe to RNA topics
    pub async fn subscribe_rna_topics(&mut self) -> Result<()> {
        let topics = vec![
            "p2pgo/rna/v1",
            "p2pgo/relay/discovery",
            "p2pgo/training/consensus",
        ];

        for topic in topics {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&gossipsub::IdentTopic::new(topic))?;
            info!("Subscribed to topic: {}", topic);
        }

        Ok(())
    }

    /// Broadcast relay discovery message
    pub async fn broadcast_discovery(&mut self) -> Result<()> {
        let discovery_rna = RNAMessage {
            id: format!("relay-discovery-{}", uuid::Uuid::new_v4()),
            source_peer: self.peer_id.to_string(),
            rna_type: RNAType::RelayDiscovery {
                addresses: self
                    .listening_addresses()
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect(),
                discovery_score: self.discovery_score,
                capabilities: vec![
                    "circuit-relay-v2".to_string(),
                    "gossipsub".to_string(),
                    "dht".to_string(),
                    "nat-traversal".to_string(),
                ],
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            quality_score: self.discovery_score,
            data: vec![],
        };

        let data = serde_cbor::to_vec(&discovery_rna)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(gossipsub::IdentTopic::new("p2pgo/relay/discovery"), data)?;

        debug!(
            "Broadcast relay discovery with score: {}",
            self.discovery_score
        );

        Ok(())
    }

    /// Main event loop
    pub async fn run(&mut self) -> Result<()> {
        let mut discovery_timer = tokio::time::interval(Duration::from_secs(30));
        let mut metrics_timer = tokio::time::interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                event = self.swarm.next() => {
                    if let Some(event) = event {
                        self.handle_event(event).await?;
                    }
                }
                _ = discovery_timer.tick() => {
                    self.broadcast_discovery().await?;
                }
                _ = metrics_timer.tick() => {
                    self.log_metrics();
                }
            }
        }
    }

    /// Handle swarm events
    async fn handle_event(&mut self, event: SwarmEvent<BootstrapBehaviourEvent>) -> Result<()> {
        match event {
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                info!("New connection from {} via {:?}", peer_id, endpoint);

                self.connected_peers.insert(
                    peer_id,
                    PeerInfo {
                        connected_at: std::time::Instant::now(),
                        last_seen: std::time::Instant::now(),
                        data_sent_kb: 0.0,
                        data_received_kb: 0.0,
                        is_relay_client: endpoint.is_relayed(),
                    },
                );

                self.metrics.total_peers_connected += 1;
                self.metrics.active_connections = self.connected_peers.len();

                // Add peer to Kademlia routing table
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, endpoint.get_remote_address().clone());
            }

            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Connection closed: {}", peer_id);
                self.connected_peers.remove(&peer_id);
                self.metrics.active_connections = self.connected_peers.len();
            }

            SwarmEvent::Behaviour(BootstrapBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                    propagation_source,
                    message_id,
                    message,
                },
            )) => {
                debug!(
                    "Received gossipsub message {} from {}",
                    message_id, propagation_source
                );

                if let Ok(rna) = serde_cbor::from_slice::<RNAMessage>(&message.data) {
                    self.handle_rna_message(rna, propagation_source).await?;
                }

                self.metrics.rna_messages_propagated += 1;
            }

            SwarmEvent::Behaviour(BootstrapBehaviourEvent::Mdns(mdns::Event::Discovered(
                peers,
            ))) => {
                for (peer_id, addr) in peers {
                    info!("Discovered peer {} at {} via mDNS", peer_id, addr);
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());
                    self.swarm.dial(addr)?;
                }
            }

            SwarmEvent::Behaviour(BootstrapBehaviourEvent::Kademlia(
                kad::Event::RoutingUpdated {
                    peer, addresses, ..
                },
            )) => {
                debug!(
                    "Kademlia routing updated for {}: {} addresses",
                    peer,
                    addresses.len()
                );
            }

            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);

                // Add our address to Kademlia
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&self.peer_id, address.clone());
            }

            _ => {}
        }

        Ok(())
    }

    /// Handle RNA messages
    async fn handle_rna_message(&mut self, rna: RNAMessage, from: PeerId) -> Result<()> {
        match &rna.rna_type {
            RNAType::SGFData { .. } | RNAType::PatternData { .. } => {
                // High-quality training data boosts our discovery score
                self.discovery_score = (self.discovery_score + 0.1 * rna.quality_score).min(2.0);
                info!(
                    "Discovery score increased to {} from training data",
                    self.discovery_score
                );
            }
            RNAType::ModelWeights {
                consensus_count, ..
            } => {
                // Consensus weights are very valuable
                if *consensus_count > 3 {
                    self.discovery_score = (self.discovery_score + 0.2).min(2.0);
                }
            }
            RNAType::RelayDiscovery { .. } => {
                // Track other relays
                debug!("Discovered relay from RNA: {}", rna.source_peer);
            }
            _ => {}
        }

        // Update peer metrics
        if let Some(peer_info) = self.connected_peers.get_mut(&from) {
            peer_info.last_seen = std::time::Instant::now();
            peer_info.data_received_kb += rna.data.len() as f32 / 1024.0;
        }

        Ok(())
    }

    /// Log network metrics
    fn log_metrics(&self) {
        let uptime = self.metrics.uptime.elapsed();
        info!(
            "Bootstrap Relay Metrics - Uptime: {:?}, Active: {}, Total: {}, RNA: {}, Score: {:.2}",
            uptime,
            self.metrics.active_connections,
            self.metrics.total_peers_connected,
            self.metrics.rna_messages_propagated,
            self.discovery_score
        );

        // Log peer details
        for (peer_id, info) in &self.connected_peers {
            let connected_for = info.connected_at.elapsed();
            debug!(
                "  Peer {}: connected {:?}, sent {:.1}KB, recv {:.1}KB, relay_client: {}",
                peer_id,
                connected_for,
                info.data_sent_kb,
                info.data_received_kb,
                info.is_relay_client
            );
        }
    }
}

/// Create and run a bootstrap relay
pub async fn run_bootstrap_relay(port: u16) -> Result<()> {
    let mut relay = BootstrapRelay::new(port).await?;

    // Subscribe to RNA topics
    relay.subscribe_rna_topics().await?;

    info!("Bootstrap relay started on:");
    for addr in relay.listening_addresses() {
        info!("  {}", addr);
    }

    // Initial discovery broadcast
    relay.broadcast_discovery().await?;

    // Run the relay
    relay.run().await
}

// UUID helper
mod uuid {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct Uuid;

    impl Uuid {
        pub fn new_v4() -> String {
            format!(
                "bootstrap-{}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            )
        }
    }
}
