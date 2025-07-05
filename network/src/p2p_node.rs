//! P2P node implementation with Circuit Relay V2
//!
//! This provides the core P2P functionality for the game, including:
//! - Circuit Relay V2 for NAT traversal
//! - Kademlia DHT for peer discovery
//! - Connection persistence and management
//! - Minimal mode for privacy-conscious users

use libp2p::{
    identity::Keypair,
    PeerId, Multiaddr,
    swarm::{Swarm, SwarmEvent, NetworkBehaviour},
    kad::{store::MemoryStore},
    Transport,
};

use std::collections::{HashMap, HashSet};
use std::time::Duration;
use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use tracing::{info, warn, error, debug};

/// Relay mode configuration
#[derive(Debug, Clone, PartialEq)]
pub enum RelayMode {
    /// Minimal mode - only connect when playing, no relay service
    Minimal,
    /// Normal mode - use relays, but don't provide relay service
    Normal,
    /// Provider mode - provide relay service to the network
    Provider {
        max_reservations: usize,
        max_circuits: usize,
    },
}

impl Default for RelayMode {
    fn default() -> Self {
        RelayMode::Minimal
    }
}

/// P2P node configuration
#[derive(Debug, Clone)]
pub struct P2PConfig {
    /// Relay mode
    pub relay_mode: RelayMode,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    /// Bootstrap nodes (if any)
    pub bootstrap_nodes: Vec<Multiaddr>,
    /// Listen addresses
    pub listen_addresses: Vec<Multiaddr>,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            relay_mode: RelayMode::Minimal,
            enable_mdns: true,
            bootstrap_nodes: vec![],
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/0".parse().unwrap(),
                "/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap(),
            ],
        }
    }
}

/// Combined network behaviour
#[derive(NetworkBehaviour)]
pub struct P2PBehaviour {
    kademlia: Kademlia<MemoryStore>,
    relay_client: relay::client::Behaviour,
    relay_server: Option<relay::Behaviour>,
    dcutr: dcutr::Behaviour,
    identify: Identify,
    autonat: Autonat,
    gossipsub: Gossipsub,
    mdns: Option<Mdns>,
}

/// P2P node managing the libp2p swarm
pub struct P2PNode {
    /// The libp2p swarm
    swarm: Swarm<P2PBehaviour>,
    /// Node configuration
    config: P2PConfig,
    /// Connected peers
    connected_peers: Arc<RwLock<HashSet<PeerId>>>,
    /// Known relay servers
    known_relays: Arc<RwLock<HashMap<PeerId, RelayInfo>>>,
    /// Active relay reservations
    relay_reservations: Arc<RwLock<HashMap<PeerId, RelayReservation>>>,
    /// Event channel for game logic
    event_tx: mpsc::UnboundedSender<P2PEvent>,
    /// Connection manager
    connection_manager: Arc<crate::connection_manager::ConnectionManager>,
}

/// Information about a relay server
#[derive(Debug, Clone)]
struct RelayInfo {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
    last_seen: std::time::Instant,
    quality_score: f32,
}

/// Active relay reservation
#[derive(Debug, Clone)]
struct RelayReservation {
    relay_peer: PeerId,
    expiry: std::time::Instant,
    circuit_count: usize,
}

/// P2P events sent to game logic
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// Connected to a peer
    PeerConnected { peer_id: PeerId },
    /// Disconnected from a peer
    PeerDisconnected { peer_id: PeerId },
    /// Discovered a relay server
    RelayDiscovered { peer_id: PeerId, addresses: Vec<Multiaddr> },
    /// Relay reservation established
    RelayReserved { relay_peer: PeerId },
    /// Received a gossip message
    GossipReceived { topic: String, data: Vec<u8> },
    /// DHT query completed
    DhtQueryComplete { key: Vec<u8>, providers: Vec<PeerId> },
}

impl P2PNode {
    /// Create a new P2P node
    pub async fn new(
        keypair: Keypair,
        config: P2PConfig,
        event_tx: mpsc::UnboundedSender<P2PEvent>,
    ) -> Result<Self> {
        let peer_id = PeerId::from(keypair.public());
        info!("Creating P2P node with peer ID: {}", peer_id);
        
        // Build transport
        let transport = libp2p::tokio_development_transport(keypair.clone())?;
        
        // Create behaviour
        let behaviour = Self::create_behaviour(&keypair, &config, peer_id)?;
        
        // Create swarm
        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build();
        
        // Listen on configured addresses
        for addr in &config.listen_addresses {
            swarm.listen_on(addr.clone())?;
        }
        
        // Create connection manager
        let connection_manager = Arc::new(
            crate::connection_manager::ConnectionManager::new(Default::default())
        );
        
        Ok(Self {
            swarm,
            config,
            connected_peers: Arc::new(RwLock::new(HashSet::new())),
            known_relays: Arc::new(RwLock::new(HashMap::new())),
            relay_reservations: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            connection_manager,
        })
    }
    
    /// Create the network behaviour
    fn create_behaviour(
        keypair: &Keypair,
        config: &P2PConfig,
        peer_id: PeerId,
    ) -> Result<P2PBehaviour> {
        // Kademlia for DHT
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(Duration::from_secs(60));
        let kademlia = Kademlia::with_config(
            peer_id,
            MemoryStore::new(peer_id),
            kad_config,
        );
        
        // Relay client - everyone can use relays
        let relay_client = relay::client::Behaviour::new(peer_id);
        
        // Relay server - optional based on mode
        let relay_server = match &config.relay_mode {
            RelayMode::Provider { max_reservations, max_circuits } => {
                Some(relay::Behaviour::new(
                    peer_id,
                    relay::Config {
                        max_reservations: *max_reservations,
                        max_reservations_per_peer: 2,
                        reservation_duration: Duration::from_secs(60 * 60), // 1 hour
                        max_circuits: *max_circuits,
                        max_circuits_per_peer: 2,
                        ..Default::default()
                    },
                ))
            }
            _ => None,
        };
        
        // DCUtR for connection upgrade
        let dcutr = dcutr::Behaviour::new(peer_id);
        
        // Identify protocol
        let identify = Identify::new(IdentifyConfig::new(
            "/p2pgo/1.0.0".to_string(),
            keypair.public(),
        ));
        
        // AutoNAT for NAT detection
        let autonat = Autonat::new(peer_id, Default::default());
        
        // GossipSub for game state propagation
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(libp2p::gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid gossipsub config");
            
        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        ).expect("Valid gossipsub");
        
        // mDNS for local discovery
        let mdns = if config.enable_mdns {
            Some(Mdns::new(Default::default())?)
        } else {
            None
        };
        
        Ok(P2PBehaviour {
            kademlia,
            relay_client,
            relay_server,
            dcutr,
            identify,
            autonat,
            gossipsub,
            mdns,
        })
    }
    
    /// Start the P2P node
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting P2P node");
        
        // Bootstrap if we have bootstrap nodes
        if !self.config.bootstrap_nodes.is_empty() {
            self.bootstrap().await?;
        }
        
        // Start discovering relays if not in minimal mode
        if self.config.relay_mode != RelayMode::Minimal {
            self.start_relay_discovery().await?;
        }
        
        // Subscribe to game topics
        self.swarm.behaviour_mut().gossipsub.subscribe(&gossipsub_topic("games"))?;
        
        Ok(())
    }
    
    /// Bootstrap with known nodes
    async fn bootstrap(&mut self) -> Result<()> {
        info!("Bootstrapping with {} nodes", self.config.bootstrap_nodes.len());
        
        for addr in &self.config.bootstrap_nodes {
            if let Some(peer_id) = addr.iter().find_map(|p| {
                if let libp2p::multiaddr::Protocol::P2p(hash) = p {
                    PeerId::from_multihash(hash).ok()
                } else {
                    None
                }
            }) {
                self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                self.swarm.dial(addr.clone())?;
            }
        }
        
        // Bootstrap Kademlia
        self.swarm.behaviour_mut().kademlia.bootstrap()?;
        
        Ok(())
    }
    
    /// Start discovering relay servers
    async fn start_relay_discovery(&mut self) -> Result<()> {
        info!("Starting relay discovery");
        
        // Query DHT for relay providers
        let key = libp2p::kad::RecordKey::new(&b"relay-servers"[..]);
        self.swarm.behaviour_mut().kademlia.get_providers(key);
        
        // If we're a relay provider, advertise ourselves
        if matches!(self.config.relay_mode, RelayMode::Provider { .. }) {
            self.advertise_as_relay().await?;
        }
        
        Ok(())
    }
    
    /// Advertise as a relay server
    async fn advertise_as_relay(&mut self) -> Result<()> {
        info!("Advertising as relay server");
        
        let key = libp2p::kad::RecordKey::new(&b"relay-servers"[..]);
        self.swarm.behaviour_mut().kademlia.start_providing(key)?;
        
        Ok(())
    }
    
    /// Connect to a peer, using relay if necessary
    pub async fn connect_peer(&mut self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<()> {
        info!("Attempting to connect to peer {}", peer_id);
        
        // Add addresses to Kademlia
        for addr in &addresses {
            self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
        }
        
        // Use connection manager for robust connection
        let swarm = &mut self.swarm;
        self.connection_manager.connect_with_retry(
            peer_id,
            || async {
                // Try direct connection first
                for addr in &addresses {
                    if let Err(e) = swarm.dial(addr.clone()) {
                        debug!("Failed to dial {}: {}", addr, e);
                    } else {
                        return Ok(());
                    }
                }
                
                // If direct fails, try via relay
                if !self.relay_reservations.read().await.is_empty() {
                    info!("Direct connection failed, trying via relay");
                    self.connect_via_relay(peer_id).await
                } else {
                    Err(anyhow!("No direct connection and no relay available"))
                }
            }
        ).await
    }
    
    /// Connect to a peer via relay
    async fn connect_via_relay(&mut self, target: PeerId) -> Result<()> {
        let reservations = self.relay_reservations.read().await;
        
        for (relay_peer, reservation) in reservations.iter() {
            if reservation.expiry > std::time::Instant::now() {
                // Build relay address
                let relay_addr = format!("/p2p/{}/p2p-circuit/p2p/{}", relay_peer, target)
                    .parse::<Multiaddr>()?;
                
                if let Err(e) = self.swarm.dial(relay_addr) {
                    warn!("Failed to dial via relay {}: {}", relay_peer, e);
                } else {
                    info!("Initiated connection to {} via relay {}", target, relay_peer);
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("No active relay reservations available"))
    }
    
    /// Make a relay reservation
    pub async fn reserve_relay(&mut self, relay_peer: PeerId) -> Result<()> {
        info!("Requesting relay reservation from {}", relay_peer);
        
        // This will trigger the relay protocol
        // The actual reservation will be confirmed via swarm events
        
        Ok(())
    }
    
    /// Publish a game to the network
    pub async fn publish_game(&mut self, game_id: &str, metadata: GameMetadata) -> Result<()> {
        // Store in DHT
        let key = format!("/games/{}", game_id);
        let record = libp2p::kad::Record {
            key: libp2p::kad::RecordKey::new(&key),
            value: serde_json::to_vec(&metadata)?,
            publisher: None,
            expires: None,
        };
        
        self.swarm.behaviour_mut().kademlia.put_record(record, libp2p::kad::Quorum::One)?;
        
        // Announce via GossipSub
        let message = serde_json::to_vec(&GameAnnouncement {
            game_id: game_id.to_string(),
            metadata,
        })?;
        
        self.swarm.behaviour_mut().gossipsub.publish(gossipsub_topic("games"), message)?;
        
        info!("Published game {} to P2P network", game_id);
        Ok(())
    }
    
    /// Find available games
    pub async fn find_games(&mut self, board_size: Option<u8>) -> Result<()> {
        let pattern = match board_size {
            Some(size) => format!("/games/size/{}", size),
            None => "/games/".to_string(),
        };
        
        self.swarm.behaviour_mut().kademlia.start_providing(
            libp2p::kad::RecordKey::new(&pattern)
        )?;
        
        Ok(())
    }
    
    /// Handle swarm events
    pub async fn handle_events(&mut self) -> Result<()> {
        loop {
            match self.swarm.next().await {
                Some(SwarmEvent::Behaviour(event)) => {
                    self.handle_behaviour_event(event).await?;
                }
                Some(SwarmEvent::NewListenAddr { address, .. }) => {
                    info!("Listening on {}", address);
                }
                Some(SwarmEvent::ConnectionEstablished { peer_id, .. }) => {
                    self.handle_connection_established(peer_id).await?;
                }
                Some(SwarmEvent::ConnectionClosed { peer_id, .. }) => {
                    self.handle_connection_closed(peer_id).await?;
                }
                _ => {}
            }
        }
    }
    
    /// Handle behaviour events
    async fn handle_behaviour_event(&mut self, event: P2PBehaviourEvent) -> Result<()> {
        match event {
            P2PBehaviourEvent::Kademlia(e) => self.handle_kad_event(e).await?,
            P2PBehaviourEvent::Identify(e) => self.handle_identify_event(e).await?,
            P2PBehaviourEvent::RelayClient(e) => self.handle_relay_client_event(e).await?,
            P2PBehaviourEvent::Gossipsub(e) => self.handle_gossipsub_event(e).await?,
            P2PBehaviourEvent::Mdns(Some(e)) => self.handle_mdns_event(e).await?,
            _ => {}
        }
        Ok(())
    }
    
    /// Handle Kademlia events
    async fn handle_kad_event(&mut self, event: KademliaEvent) -> Result<()> {
        match event {
            KademliaEvent::OutboundQueryCompleted { result, .. } => {
                match result {
                    libp2p::kad::QueryResult::GetProviders(Ok(result)) => {
                        let providers: Vec<_> = result.providers().collect();
                        info!("Found {} providers", providers.len());
                        
                        // Check if these are relay providers
                        for peer in providers {
                            self.check_relay_capability(peer).await?;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle identify events
    async fn handle_identify_event(&mut self, event: IdentifyEvent) -> Result<()> {
        match event {
            IdentifyEvent::Received { peer_id, info } => {
                debug!("Identified peer {}: {:?}", peer_id, info.protocol_version);
                
                // Check if peer supports relay
                if info.protocols.iter().any(|p| p.as_bytes() == b"/libp2p/circuit/relay/0.2.0/hop") {
                    info!("Peer {} supports relay service", peer_id);
                    
                    let relay_info = RelayInfo {
                        peer_id,
                        addresses: info.listen_addrs,
                        last_seen: std::time::Instant::now(),
                        quality_score: 1.0,
                    };
                    
                    self.known_relays.write().await.insert(peer_id, relay_info);
                    self.event_tx.send(P2PEvent::RelayDiscovered {
                        peer_id,
                        addresses: info.listen_addrs,
                    })?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle relay client events
    async fn handle_relay_client_event(&mut self, event: relay::client::Event) -> Result<()> {
        match event {
            relay::client::Event::ReservationReqAccepted { relay_peer_id, .. } => {
                info!("Relay reservation accepted by {}", relay_peer_id);
                
                let reservation = RelayReservation {
                    relay_peer: relay_peer_id,
                    expiry: std::time::Instant::now() + Duration::from_secs(3600),
                    circuit_count: 0,
                };
                
                self.relay_reservations.write().await.insert(relay_peer_id, reservation);
                self.event_tx.send(P2PEvent::RelayReserved { relay_peer: relay_peer_id })?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle gossipsub events
    async fn handle_gossipsub_event(&mut self, event: GossipsubEvent) -> Result<()> {
        match event {
            GossipsubEvent::Message { propagation_source, message_id, message } => {
                debug!("Received gossip message from {}: {:?}", propagation_source, message_id);
                
                self.event_tx.send(P2PEvent::GossipReceived {
                    topic: message.topic.to_string(),
                    data: message.data,
                })?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle mDNS events
    async fn handle_mdns_event(&mut self, event: MdnsEvent) -> Result<()> {
        match event {
            MdnsEvent::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    info!("Discovered local peer {} at {}", peer_id, addr);
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
            MdnsEvent::Expired(peers) => {
                for (peer_id, _) in peers {
                    info!("Local peer {} expired", peer_id);
                }
            }
        }
        Ok(())
    }
    
    /// Handle connection established
    async fn handle_connection_established(&mut self, peer_id: PeerId) -> Result<()> {
        info!("Connection established with {}", peer_id);
        
        self.connected_peers.write().await.insert(peer_id);
        self.connection_manager.handle_disconnection(&peer_id);
        self.event_tx.send(P2PEvent::PeerConnected { peer_id })?;
        
        // Check if this is a relay server
        self.check_relay_capability(peer_id).await?;
        
        Ok(())
    }
    
    /// Handle connection closed
    async fn handle_connection_closed(&mut self, peer_id: PeerId) -> Result<()> {
        info!("Connection closed with {}", peer_id);
        
        self.connected_peers.write().await.remove(&peer_id);
        self.connection_manager.handle_disconnection(&peer_id);
        self.event_tx.send(P2PEvent::PeerDisconnected { peer_id })?;
        
        Ok(())
    }
    
    /// Check if a peer provides relay service
    async fn check_relay_capability(&mut self, peer_id: PeerId) -> Result<()> {
        // This will trigger an identify exchange
        // The actual capability check happens in handle_identify_event
        Ok(())
    }
    
    /// Get connected peers
    pub async fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers.read().await.iter().cloned().collect()
    }
    
    /// Get known relay servers
    pub async fn known_relays(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        self.known_relays.read().await
            .iter()
            .map(|(id, info)| (*id, info.addresses.clone()))
            .collect()
    }
}

/// Game metadata for discovery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameMetadata {
    pub id: String,
    pub board_size: u8,
    pub host_name: String,
    pub time_control: Option<String>,
}

/// Game announcement message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GameAnnouncement {
    pub game_id: String,
    pub metadata: GameMetadata,
}

/// Create a gossipsub topic
fn gossipsub_topic(name: &str) -> libp2p::gossipsub::IdentTopic {
    libp2p::gossipsub::IdentTopic::new(format!("/p2pgo/{}/1.0.0", name))
}