use crate::{
    behaviour::{Event, P2PGoBehaviour},
    bootstrap::{Bootstrap, BootstrapConfig},
    rna::{RNAMessage, RNAType},
};
use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::{identity::Keypair, noise, tcp, yamux, PeerId, Swarm, SwarmBuilder};
use std::{collections::HashMap, time::Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid;

/// Connection metrics for logging
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub peer_id: PeerId,
    pub connected_at: Instant,
    pub connection_type: ConnectionType,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum ConnectionType {
    Direct,
    Relayed,
    Local,
}

pub struct RelayNode {
    swarm: Swarm<P2PGoBehaviour>,
    /// RNA message channel
    #[allow(dead_code)]
    rna_tx: mpsc::Sender<RNAMessage>,
    rna_rx: mpsc::Receiver<RNAMessage>,
    /// Connection metrics
    connections: HashMap<PeerId, ConnectionMetrics>,
    /// Bootstrap helper
    bootstrap: Bootstrap,
}

impl RelayNode {
    /// Create a new relay node
    pub fn new(keypair: Keypair) -> Result<Self> {
        let local_peer_id = PeerId::from(keypair.public());
        info!("Creating relay node with peer ID: {}", local_peer_id);

        // Create swarm using the new SwarmBuilder API
        let mut swarm = SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let peer_id = PeerId::from(key.public());
                Ok(P2PGoBehaviour::new(peer_id, key)?)
            })?
            .build();

        // Listen on all interfaces
        let listen_addr = format!("/ip4/0.0.0.0/tcp/0");
        swarm
            .listen_on(listen_addr.parse()?)
            .context("Failed to start listening")?;

        let (rna_tx, rna_rx) = mpsc::channel(100);

        Ok(Self {
            swarm,
            rna_tx,
            rna_rx,
            connections: HashMap::new(),
            bootstrap: Bootstrap::new(BootstrapConfig::default()),
        })
    }

    /// Get our peer ID
    pub fn peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    /// Get our listening addresses
    pub fn listening_addresses(&self) -> Vec<String> {
        self.swarm
            .listeners()
            .map(|addr| addr.to_string())
            .collect()
    }

    /// Bootstrap the node
    pub async fn bootstrap(&mut self) -> Result<()> {
        let result = self.bootstrap.bootstrap(&mut self.swarm).await?;
        info!(
            "Bootstrap complete. Local peer ID: {}",
            result.local_peer_id
        );

        // Subscribe to core topics
        let topics = vec!["p2pgo/games/v1", "p2pgo/rna/v1", "p2pgo/lobby/v1"];

        for topic in topics {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&libp2p::gossipsub::IdentTopic::new(topic))?;
            info!("Subscribed to topic: {}", topic);
        }

        Ok(())
    }

    /// Connect directly to another peer
    pub async fn connect_to_peer(&mut self, addr: libp2p::Multiaddr) -> Result<()> {
        info!("Connecting to peer: {}", addr);
        self.bootstrap.quick_connect(&mut self.swarm, addr).await
    }

    /// Broadcast RNA message (game data)
    pub async fn broadcast_rna(&mut self, rna: RNAMessage) -> Result<()> {
        let data = serde_cbor::to_vec(&rna)?;

        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(libp2p::gossipsub::IdentTopic::new("p2pgo/rna/v1"), data)?;

        info!("Broadcast RNA message: {:?}", rna.rna_type);
        Ok(())
    }

    /// Create RNA from SGF file
    pub fn create_sgf_rna(&self, sgf_content: String, move_range: (usize, usize)) -> RNAMessage {
        RNAMessage {
            id: uuid::Uuid::new_v4().to_string(),
            rna_type: RNAType::SGFData {
                sgf_content,
                move_range,
                player_ranks: ("1d".to_string(), "1d".to_string()), // TODO: extract from SGF
            },
            data: vec![],
            timestamp: chrono::Utc::now().timestamp() as u64,
            source_peer: self.peer_id().to_string(),
            quality_score: 0.8, // TODO: calculate quality score
        }
    }

    /// Process network events
    pub async fn handle_events(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                // Handle swarm events
                event = self.swarm.next() => {
                    if let Some(event) = event {
                    match event {
                        libp2p::swarm::SwarmEvent::Behaviour(Event::Identify(e)) => {
                            self.handle_identify_event(e);
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(Event::Gossipsub(e)) => {
                            self.handle_gossipsub_event(e).await?;
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(Event::Mdns(e)) => {
                            self.handle_mdns_event(e)?;
                        }
                        libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                            self.handle_connection_established(peer_id, endpoint);
                        }
                        libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            self.handle_connection_closed(peer_id);
                        }
                        _ => {}
                    }
                    }
                }

                // Handle RNA messages to broadcast
                Some(rna) = self.rna_rx.recv() => {
                    self.broadcast_rna(rna).await?;
                }
            }
        }
    }

    fn handle_identify_event(&mut self, event: libp2p::identify::Event) {
        match event {
            libp2p::identify::Event::Received { peer_id, info } => {
                debug!("Identified peer {}: {:?}", peer_id, info);

                // Add addresses to Kademlia
                for addr in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }
            _ => {}
        }
    }

    async fn handle_gossipsub_event(&mut self, event: libp2p::gossipsub::Event) -> Result<()> {
        match event {
            libp2p::gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            } => {
                if message.topic == libp2p::gossipsub::IdentTopic::new("p2pgo/rna/v1").hash() {
                    match serde_cbor::from_slice::<RNAMessage>(&message.data) {
                        Ok(rna) => {
                            info!(
                                "Received RNA from {}: {:?}",
                                propagation_source, rna.rna_type
                            );

                            // Update connection metrics
                            if let Some(metrics) = self.connections.get_mut(&propagation_source) {
                                metrics.latency_ms =
                                    Some(metrics.connected_at.elapsed().as_millis() as u64);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to decode RNA message: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_mdns_event(&mut self, event: libp2p::mdns::Event) -> Result<()> {
        match event {
            libp2p::mdns::Event::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    info!("Discovered local peer {} at {}", peer_id, addr);

                    // Connect to discovered peer
                    if self.swarm.dial(addr.clone()).is_ok() {
                        self.connections.insert(
                            peer_id,
                            ConnectionMetrics {
                                peer_id,
                                connected_at: Instant::now(),
                                connection_type: ConnectionType::Local,
                                latency_ms: None,
                            },
                        );
                    }
                }
            }
            libp2p::mdns::Event::Expired(peers) => {
                for (peer_id, _) in peers {
                    info!("mDNS peer expired: {}", peer_id);
                }
            }
        }
        Ok(())
    }

    fn handle_connection_established(
        &mut self,
        peer_id: PeerId,
        endpoint: libp2p::core::ConnectedPoint,
    ) {
        let connection_type = match &endpoint {
            libp2p::core::ConnectedPoint::Dialer { address, .. } => {
                if address.to_string().contains("/p2p-circuit") {
                    ConnectionType::Relayed
                } else {
                    ConnectionType::Direct
                }
            }
            libp2p::core::ConnectedPoint::Listener { .. } => ConnectionType::Direct,
        };

        info!(
            "Connection established with {} ({:?})",
            peer_id, connection_type
        );

        self.connections.insert(
            peer_id,
            ConnectionMetrics {
                peer_id,
                connected_at: Instant::now(),
                connection_type,
                latency_ms: None,
            },
        );
    }

    fn handle_connection_closed(&mut self, peer_id: PeerId) {
        if let Some(metrics) = self.connections.remove(&peer_id) {
            let duration = metrics.connected_at.elapsed();
            info!(
                "Connection closed with {} after {:?} ({:?})",
                peer_id, duration, metrics.connection_type
            );
        }
    }

    /// Get current connection metrics
    pub fn get_connection_metrics(&self) -> Vec<ConnectionMetrics> {
        self.connections.values().cloned().collect()
    }
}
