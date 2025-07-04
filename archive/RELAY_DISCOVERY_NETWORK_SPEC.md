# Relay Discovery & P2P Network Architecture Specification

## Overview

RNA propagation directly drives relay discovery. When you create valuable training data (mRNA), spreading it increases your relay's reputation and discoverability, creating natural incentives for knowledge sharing.

## 1. Core Networking Stack

### libp2p Circuit Relay V2 Architecture

```rust
// network/src/lib.rs
use libp2p::{
    core::{multiaddr::Multiaddr, PeerId},
    dcutr, gossipsub, identify, kad,
    relay::{v2::relay, v2::client},
    swarm::{NetworkBehaviour, SwarmBuilder},
    tcp, yamux, noise,
};

/// Main network behavior combining all protocols
#[derive(NetworkBehaviour)]
pub struct P2PGoBehaviour {
    /// Relay client for NAT traversal
    relay_client: client::Client,
    /// DHT for peer discovery and Elo rankings
    kademlia: kad::Kademlia<kad::store::MemoryStore>,
    /// Gossipsub for RNA propagation
    gossipsub: gossipsub::Gossipsub,
    /// Direct connection upgrade through relay
    dcutr: dcutr::Behaviour,
    /// Peer identification
    identify: identify::Behaviour,
    /// Custom RNA discovery protocol
    rna_discovery: RNADiscovery,
}

// network/src/relay_node.rs
pub struct RelayNode {
    /// Our peer ID (derived from keypair)
    peer_id: PeerId,
    /// Swarm handle
    swarm: Swarm<P2PGoBehaviour>,
    /// RNA message queue
    rna_queue: Arc<RwLock<VecDeque<RNAMessage>>>,
    /// Discovery metrics
    discovery_metrics: DiscoveryMetrics,
}

impl RelayNode {
    pub fn new(keypair: Keypair) -> Result<Self, Error> {
        let peer_id = PeerId::from(keypair.public());
        
        // Configure transport with relay
        let transport = {
            let tcp = tcp::async_io::Transport::new(tcp::Config::default());
            let transport = relay::client::new(peer_id);
            
            transport
                .or_transport(tcp)
                .upgrade(noise::NoiseAuthenticated::xx(&keypair)?)
                .multiplex(yamux::YamuxConfig::default())
                .boxed()
        };
        
        // Create swarm
        let mut swarm = SwarmBuilder::with_async_std_executor(
            transport,
            P2PGoBehaviour::new(peer_id, keypair.clone())?,
            peer_id,
        ).build();
        
        // Listen on all interfaces (handles NAT better)
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        swarm.listen_on("/ip6/::/tcp/0".parse()?)?;
        
        Ok(Self {
            peer_id,
            swarm,
            rna_queue: Arc::new(RwLock::new(VecDeque::new())),
            discovery_metrics: Default::default(),
        })
    }
}
```

## 2. RNA-Driven Discovery Protocol

```rust
// network/src/rna_discovery.rs
use libp2p::request_response::{
    RequestResponse, RequestResponseCodec, RequestResponseEvent,
    RequestResponseMessage, ProtocolSupport,
};

/// RNA Discovery Protocol - spreading RNA increases discoverability
pub struct RNADiscovery {
    /// Tracks RNA propagation paths
    propagation_graph: HashMap<RNAHash, PropagationPath>,
    /// Discovery reputation based on RNA quality
    discovery_reputation: HashMap<PeerId, f64>,
    /// Active RNA broadcasts
    active_broadcasts: HashMap<RNAHash, BroadcastInfo>,
}

#[derive(Clone, Debug)]
pub struct PropagationPath {
    /// Original creator of the RNA
    pub origin: PeerId,
    /// Relays that have forwarded this RNA
    pub path: Vec<(PeerId, Instant)>,
    /// Quality score (updated by receivers)
    pub quality_score: f64,
    /// Number of successful uses
    pub usage_count: u32,
}

impl RNADiscovery {
    /// Broadcast new mRNA to increase discoverability
    pub async fn broadcast_rna(&mut self, rna: TrainingRNA) -> Result<(), Error> {
        let rna_hash = rna.hash();
        
        // Initialize propagation tracking
        self.propagation_graph.insert(rna_hash, PropagationPath {
            origin: self.peer_id,
            path: vec![(self.peer_id, Instant::now())],
            quality_score: 1.0,
            usage_count: 0,
        });
        
        // Create discovery-enhanced message
        let discovery_msg = RNADiscoveryMessage {
            rna,
            origin_reputation: self.get_reputation(),
            suggested_routes: self.calculate_optimal_routes(&rna_hash),
            discovery_incentive: self.calculate_incentive(&rna),
        };
        
        // Gossip to peers with discovery metadata
        self.gossipsub.publish(
            Topic::new("rna_discovery"),
            discovery_msg.encode(),
        )?;
        
        Ok(())
    }
    
    /// Process incoming RNA and update discovery graph
    pub fn process_rna(&mut self, rna: TrainingRNA, from: PeerId) {
        let rna_hash = rna.hash();
        
        // Update propagation path
        if let Some(path) = self.propagation_graph.get_mut(&rna_hash) {
            path.path.push((from, Instant::now()));
        } else {
            // New RNA discovered through network
            self.propagation_graph.insert(rna_hash, PropagationPath {
                origin: rna.source_dna.peer_id,
                path: vec![(from, Instant::now())],
                quality_score: 0.5, // Initial skepticism
                usage_count: 0,
            });
        }
        
        // Boost discoverer's reputation
        *self.discovery_reputation.entry(from).or_insert(0.0) += 0.1;
        
        // If RNA proves valuable, boost origin's discoverability
        if self.evaluate_rna_quality(&rna) > 0.7 {
            self.boost_origin_discovery(&rna.source_dna.peer_id);
        }
    }
    
    /// Calculate discovery incentive for RNA
    fn calculate_incentive(&self, rna: &TrainingRNA) -> DiscoveryIncentive {
        match &rna.rna_type {
            RNAType::MessengerRNA { quality_score, .. } => {
                DiscoveryIncentive {
                    relay_boost: quality_score * 2.0, // High quality = better discovery
                    ttl_extension: (*quality_score * 5.0) as u8,
                    priority_routing: *quality_score > 0.8,
                }
            }
            _ => DiscoveryIncentive::default(),
        }
    }
}
```

## 3. DHT-Based Elo Discovery

```rust
// network/src/elo_dht.rs
use libp2p::kad::{
    record::{Key, Record},
    Quorum, QueryResult,
};
use skillratings::{trueskill::TrueSkillRating, Outcomes};

/// Enhanced Elo system using TrueSkill for better precision
pub struct EloDHT {
    /// Kademlia DHT handle
    kad: kad::Kademlia<kad::store::MemoryStore>,
    /// Local Elo cache
    elo_cache: HashMap<PeerId, TrueSkillRating>,
    /// Elo update queue
    update_queue: VecDeque<EloUpdate>,
}

#[derive(Clone, Debug)]
pub struct EloUpdate {
    pub player1: PeerId,
    pub player2: PeerId,
    pub outcome: Outcomes,
    pub game_quality: f64, // Affects rating change magnitude
}

impl EloDHT {
    /// Store Elo in DHT with peer ID as key
    pub async fn publish_elo(&mut self, peer_id: PeerId, rating: TrueSkillRating) {
        let key = Key::from(peer_id.to_bytes());
        let value = EloRecord {
            rating: rating.rating,
            uncertainty: rating.uncertainty,
            games_played: self.get_game_count(&peer_id),
            last_updated: SystemTime::now(),
            relay_performance: self.calculate_relay_performance(&peer_id),
        };
        
        let record = Record {
            key,
            value: value.encode(),
            publisher: Some(self.peer_id),
            expires: Some(Instant::now() + Duration::from_secs(86400)), // 24h
        };
        
        self.kad.put_record(record, Quorum::All).await;
    }
    
    /// Find opponents by Elo range
    pub async fn find_opponents(&mut self, 
        target_elo: f64, 
        range: f64
    ) -> Vec<(PeerId, TrueSkillRating)> {
        let min_elo = target_elo - range;
        let max_elo = target_elo + range;
        
        // Use DHT provider records for Elo range
        let key_prefix = self.elo_range_to_key_prefix(min_elo, max_elo);
        
        let providers = self.kad
            .get_providers(key_prefix)
            .await
            .unwrap_or_default();
        
        // Filter and sort by Elo proximity
        let mut candidates: Vec<_> = providers
            .into_iter()
            .filter_map(|peer_id| {
                self.elo_cache.get(&peer_id)
                    .filter(|rating| {
                        rating.rating >= min_elo && rating.rating <= max_elo
                    })
                    .map(|rating| (peer_id, *rating))
            })
            .collect();
        
        candidates.sort_by(|a, b| {
            let dist_a = (a.1.rating - target_elo).abs();
            let dist_b = (b.1.rating - target_elo).abs();
            dist_a.partial_cmp(&dist_b).unwrap()
        });
        
        candidates
    }
}

/// Using TrueSkill for better rating precision
// Cargo.toml addition:
// skillratings = "0.25"
```

## 4. Bootstrap & Initial Discovery

```rust
// network/src/bootstrap.rs
/// Bootstrap nodes run by community (not centralized)
pub const BOOTSTRAP_NODES: &[&str] = &[
    // Community-run bootstrap relays
    "/dnsaddr/bootstrap1.p2pgo.net",
    "/dnsaddr/bootstrap2.p2pgo.net", 
    "/ip4/168.119.236.241/tcp/4001/p2p/12D3KooWQYV9...",
];

pub struct BootstrapStrategy {
    /// mDNS for local network discovery
    mdns: Mdns,
    /// Bootstrap nodes for initial connection
    bootstrap_peers: Vec<Multiaddr>,
    /// Rendezvous points for NAT traversal
    rendezvous_points: Vec<PeerId>,
}

impl BootstrapStrategy {
    /// Multi-stage bootstrap process
    pub async fn bootstrap(&mut self) -> Result<Vec<PeerId>, Error> {
        let mut discovered_peers = Vec::new();
        
        // Stage 1: Try mDNS for local peers
        info!("Stage 1: Searching local network via mDNS...");
        if let Ok(local_peers) = self.discover_mdns().await {
            discovered_peers.extend(local_peers);
            if !discovered_peers.is_empty() {
                info!("Found {} local peers", discovered_peers.len());
            }
        }
        
        // Stage 2: Connect to bootstrap nodes
        if discovered_peers.is_empty() {
            info!("Stage 2: Connecting to bootstrap nodes...");
            for addr in &self.bootstrap_peers {
                match self.swarm.dial(addr.clone()) {
                    Ok(_) => info!("Connected to bootstrap: {}", addr),
                    Err(e) => warn!("Failed to connect to {}: {}", addr, e),
                }
            }
        }
        
        // Stage 3: Use DHT to find peers
        info!("Stage 3: DHT discovery...");
        self.kad.bootstrap()?;
        
        // Stage 4: Rendezvous discovery for NAT'd peers
        info!("Stage 4: Rendezvous point discovery...");
        for rendezvous in &self.rendezvous_points {
            self.discover_via_rendezvous(*rendezvous).await?;
        }
        
        Ok(discovered_peers)
    }
    
    /// Create relay reservation for NAT traversal
    pub async fn setup_relay_circuit(&mut self, relay: PeerId) -> Result<(), Error> {
        // Reserve slot on relay
        self.swarm
            .behaviour_mut()
            .relay_client
            .reserve(relay)
            .await?;
        
        info!("Reserved relay slot on {}", relay);
        
        // Advertise our relayed address
        let relayed_addr = Multiaddr::empty()
            .with(Protocol::P2p(relay.into()))
            .with(Protocol::P2pCircuit)
            .with(Protocol::P2p(self.peer_id.into()));
        
        self.swarm.add_external_address(relayed_addr);
        
        Ok(())
    }
}
```

## 5. Firewall Traversal Strategy

```rust
// network/src/nat_traversal.rs
pub struct NATTraversal {
    /// UPnP for automatic port forwarding
    upnp_client: Option<igd::Gateway>,
    /// STUN for external address discovery
    stun_client: StunClient,
    /// Relay fallback strategy
    relay_strategy: RelayStrategy,
}

impl NATTraversal {
    /// Automatic NAT traversal with multiple strategies
    pub async fn setup(&mut self) -> Result<NATStatus, Error> {
        // Try UPnP first (works on many home routers)
        if let Ok(gateway) = igd::search_gateway(Default::default()) {
            match gateway.add_port(
                igd::PortMappingProtocol::TCP,
                self.local_port,
                SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, self.local_port),
                86400, // 24 hours
                "P2P Go Game",
            ) {
                Ok(()) => {
                    info!("UPnP port forwarding successful");
                    return Ok(NATStatus::Open);
                }
                Err(e) => warn!("UPnP failed: {}", e),
            }
        }
        
        // Try STUN to determine NAT type
        let stun_result = self.stun_client.discover_nat_type().await?;
        match stun_result.nat_type {
            NATType::None | NATType::FullCone => {
                return Ok(NATStatus::Open);
            }
            NATType::Symmetric => {
                warn!("Symmetric NAT detected - will use relay");
                return Ok(NATStatus::Relayed);
            }
            _ => {
                // Try hole punching for restricted NATs
                if self.try_hole_punching().await? {
                    return Ok(NATStatus::Restricted);
                }
            }
        }
        
        // Fallback to relay
        Ok(NATStatus::Relayed)
    }
    
    /// Smart relay selection based on latency and capacity
    pub async fn select_best_relay(&self) -> Result<PeerId, Error> {
        let candidates = self.discover_relay_nodes().await?;
        
        // Test latency to each relay
        let mut relay_metrics = Vec::new();
        for relay in candidates {
            let latency = self.measure_latency(&relay).await?;
            let capacity = self.query_relay_capacity(&relay).await?;
            
            relay_metrics.push((relay, latency, capacity));
        }
        
        // Select relay with best score (low latency + high capacity)
        relay_metrics.sort_by_key(|(_, latency, capacity)| {
            (latency.as_millis() as f64 / capacity.available_slots as f64) as u64
        });
        
        Ok(relay_metrics[0].0)
    }
}
```

## 6. Professional P2P Architecture

```rust
// network/src/architecture.rs
/// Layered P2P architecture
pub struct P2PArchitecture {
    /// Layer 1: Transport (TCP, QUIC, WebRTC)
    transport_layer: TransportLayer,
    /// Layer 2: Security (Noise protocol)
    security_layer: SecurityLayer,
    /// Layer 3: Multiplexing (Yamux)
    multiplex_layer: MultiplexLayer,
    /// Layer 4: Application protocols
    application_layer: ApplicationLayer,
}

pub struct ApplicationLayer {
    /// Game protocol
    game_protocol: GameProtocol,
    /// RNA gossip protocol
    rna_protocol: RNAProtocol,
    /// Discovery protocol
    discovery_protocol: DiscoveryProtocol,
    /// Consensus protocol
    consensus_protocol: ConsensusProtocol,
}

/// Connection quality monitoring
pub struct ConnectionMonitor {
    /// Per-peer connection metrics
    peer_metrics: HashMap<PeerId, ConnectionMetrics>,
    /// Network health score
    health_score: f64,
}

impl ConnectionMonitor {
    /// Adaptive protocol selection based on connection quality
    pub fn select_protocol(&self, peer: &PeerId) -> Protocol {
        let metrics = self.peer_metrics.get(peer).unwrap();
        
        match (metrics.packet_loss, metrics.latency) {
            (loss, _) if loss > 0.05 => Protocol::ReliableUDP,
            (_, latency) if latency > Duration::from_millis(200) => Protocol::QUIC,
            _ => Protocol::TCP,
        }
    }
}
```

## 7. Module Organization

```
p2pgo/
├── network/
│   ├── src/
│   │   ├── lib.rs              # Main network module
│   │   ├── relay_node.rs       # Relay node implementation
│   │   ├── rna_discovery.rs    # RNA-driven discovery
│   │   ├── elo_dht.rs          # DHT-based Elo system
│   │   ├── bootstrap.rs        # Bootstrap strategies
│   │   ├── nat_traversal.rs    # NAT/firewall traversal
│   │   ├── architecture.rs     # P2P architecture
│   │   ├── protocols/
│   │   │   ├── game.rs         # Game protocol
│   │   │   ├── rna.rs          # RNA propagation
│   │   │   └── consensus.rs    # Territory consensus
│   │   └── transports/
│   │       ├── tcp.rs          # TCP transport
│   │       ├── quic.rs         # QUIC transport
│   │       └── webrtc.rs       # WebRTC for browsers
│   └── Cargo.toml
```

## 8. Discovery Flow Example

```rust
// Example: How first relay finds second
async fn discovery_example() {
    // 1. Create first relay
    let relay1 = RelayNode::new(generate_keypair())?;
    
    // 2. Bootstrap process
    let bootstrap = BootstrapStrategy::new();
    let initial_peers = bootstrap.bootstrap().await?;
    
    // 3. Create valuable mRNA
    let training_rna = create_training_rna(game_data);
    
    // 4. Broadcast increases discoverability
    relay1.rna_discovery.broadcast_rna(training_rna).await?;
    
    // 5. Second relay discovers first through RNA
    let relay2 = RelayNode::new(generate_keypair())?;
    
    // 6. Relay2 receives RNA, discovers relay1 as quality source
    relay2.on_rna_received(|rna, from| {
        if rna.quality_score > 0.8 {
            // High quality RNA source - prioritize connection
            relay2.prioritize_peer(from);
        }
    });
    
    // 7. Direct connection upgrade through relay
    relay2.dcutr.upgrade_connection(relay1.peer_id).await?;
}
```

This architecture ensures:
- RNA creators are rewarded with better discoverability
- Multiple fallback strategies for NAT traversal
- Professional layered architecture
- No centralized dependencies
- Natural incentives for network participation