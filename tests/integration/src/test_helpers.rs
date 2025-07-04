use anyhow::Result;
use libp2p::{
    core::transport::upgrade,
    gossipsub, identity,
    kad::{self, store::MemoryStore},
    mdns, noise, relay,
    swarm::{NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info};

use p2pgo_core::{Board, Color, Move, Point};
use p2pgo_network::rna::{RNAMessage, RNAType};
use p2pgo_neural::{PolicyNetwork, ValueNetwork};
use p2pgo_sgf::parser::parse_sgf;

/// Test relay node with simplified configuration
pub struct TestRelay {
    pub swarm: Swarm<TestBehaviour>,
    pub peer_id: PeerId,
    pub rna_receiver: mpsc::Receiver<RNAMessage>,
    pub rna_sender: mpsc::Sender<RNAMessage>,
    pub connections: Arc<Mutex<HashMap<PeerId, ConnectionInfo>>>,
}

#[derive(Clone)]
pub struct ConnectionInfo {
    pub connection_type: ConnectionType,
    pub latency: Duration,
    pub bandwidth_kbps: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionType {
    Direct,
    Relayed,
    Local,
}

#[derive(NetworkBehaviour)]
pub struct TestBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub relay_client: relay::client::Behaviour,
}

impl TestRelay {
    /// Create a new test relay on the specified port
    pub async fn new(port: u16) -> Result<Self> {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        
        info!("Creating test relay with peer_id: {}", peer_id);
        
        // Create transport
        let transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&keypair)?)
            .multiplex(yamux::Config::default())
            .boxed();
        
        // Create behaviours
        let gossipsub = create_gossipsub_behaviour(&keypair)?;
        let kademlia = create_kademlia_behaviour(peer_id);
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;
        let relay_client = relay::client::Behaviour::new(peer_id);
        
        let behaviour = TestBehaviour {
            gossipsub,
            kademlia,
            mdns,
            relay_client,
        };
        
        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build();
        
        // Listen on specified port
        swarm.listen_on(format!("/ip4/127.0.0.1/tcp/{}", port).parse()?)?;
        
        let (rna_sender, rna_receiver) = mpsc::channel(100);
        
        Ok(TestRelay {
            swarm,
            peer_id,
            rna_receiver,
            rna_sender,
            connections: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Create a relay that acts as a relay server
    pub async fn new_relay_server(port: u16) -> Result<Self> {
        let mut relay = Self::new(port).await?;
        // Additional relay server configuration if needed
        relay.swarm.behaviour_mut().gossipsub.subscribe(&gossipsub::IdentTopic::new("p2pgo/relay/v1"))?;
        Ok(relay)
    }
    
    /// Create a relay behind NAT (only binds to localhost)
    pub async fn new_behind_nat(port: u16) -> Result<Self> {
        let mut relay = Self::new(port).await?;
        // Simulate NAT by only listening on localhost
        relay.swarm.listen_on(format!("/ip4/127.0.0.1/tcp/{}", port).parse()?)?;
        Ok(relay)
    }
    
    /// Get the peer ID
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }
    
    /// Get listening addresses
    pub fn listening_addresses(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }
    
    /// Connect to a peer at the given address
    pub async fn connect_to_peer(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm.dial(addr)?;
        Ok(())
    }
    
    /// Wait for a specific peer to be discovered
    pub async fn wait_for_peer(&mut self, target: PeerId, timeout: Duration) -> bool {
        let start = tokio::time::Instant::now();
        
        while start.elapsed() < timeout {
            // Check if already connected
            if self.swarm.is_connected(&target) {
                return true;
            }
            
            // Process swarm events
            tokio::select! {
                event = self.swarm.next() => {
                    if let Some(event) = event {
                        self.handle_swarm_event(event).await;
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
        }
        
        false
    }
    
    /// Check if connected to a peer
    pub async fn is_connected_to(&self, peer: PeerId) -> bool {
        self.swarm.is_connected(&peer)
    }
    
    /// Get connection type to a peer
    pub async fn get_connection_type(&self, peer: PeerId) -> ConnectionType {
        let connections = self.connections.lock().await;
        connections.get(&peer)
            .map(|info| info.connection_type.clone())
            .unwrap_or(ConnectionType::Direct)
    }
    
    /// Measure latency to a peer
    pub async fn measure_latency(&self, _peer: PeerId) -> Duration {
        // Simplified for testing - would implement ping protocol
        Duration::from_millis(5)
    }
    
    /// Subscribe to RNA topic
    pub async fn subscribe_rna(&mut self) -> Result<()> {
        self.swarm.behaviour_mut().gossipsub
            .subscribe(&gossipsub::IdentTopic::new("p2pgo/rna/v1"))?;
        Ok(())
    }
    
    /// Create RNA from SGF content
    pub fn create_sgf_rna(&self, sgf_content: String, move_range: (usize, usize)) -> RNAMessage {
        RNAMessage {
            id: format!("test-rna-{}", uuid::Uuid::new_v4()),
            source_peer: self.peer_id.to_string(),
            rna_type: RNAType::SGFData {
                sgf_content,
                move_range,
                player_ranks: ("7k".to_string(), "4k".to_string()),
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            quality_score: 0.8,
            data: vec![], // Would be filled with actual data
        }
    }
    
    /// Broadcast RNA message
    pub async fn broadcast_rna(&mut self, rna: RNAMessage) -> Result<()> {
        let data = serde_cbor::to_vec(&rna)?;
        self.swarm.behaviour_mut().gossipsub
            .publish(gossipsub::IdentTopic::new("p2pgo/rna/v1"), data)?;
        Ok(())
    }
    
    /// Wait for RNA message
    pub async fn wait_for_rna(&mut self, timeout: Duration) -> Option<RNAMessage> {
        tokio::time::timeout(timeout, self.rna_receiver.recv()).await.ok()?
    }
    
    /// Get subscribed topics
    pub async fn get_subscribed_topics(&self) -> Vec<String> {
        self.swarm.behaviour().gossipsub
            .topics()
            .map(|t| t.to_string())
            .collect()
    }
    
    /// Handle swarm events
    async fn handle_swarm_event(&mut self, event: SwarmEvent<TestBehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.connections.lock().await.insert(peer_id, ConnectionInfo {
                    connection_type: ConnectionType::Direct,
                    latency: Duration::from_millis(5),
                    bandwidth_kbps: 1000.0,
                });
            }
            SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: _,
                message_id: _,
                message,
            })) => {
                if let Ok(rna) = serde_cbor::from_slice::<RNAMessage>(&message.data) {
                    debug!("Received RNA message: {}", rna.id);
                    let _ = self.rna_sender.send(rna).await;
                }
            }
            SwarmEvent::Behaviour(TestBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, addr) in peers {
                    info!("Discovered peer {} at {}", peer_id, addr);
                    self.swarm.dial(addr).ok();
                }
            }
            _ => {}
        }
    }
}

fn create_gossipsub_behaviour(keypair: &identity::Keypair) -> Result<gossipsub::Behaviour> {
    let message_id_fn = |message: &gossipsub::Message| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&message.data, &mut hasher);
        gossipsub::MessageId::from(std::hash::Hasher::finish(&hasher).to_string())
    };
    
    let config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|e| anyhow::anyhow!("Invalid gossipsub config: {}", e))?;
    
    Ok(gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        config,
    )?)
}

fn create_kademlia_behaviour(peer_id: PeerId) -> kad::Behaviour<MemoryStore> {
    let store = MemoryStore::new(peer_id);
    let mut config = kad::Config::default();
    config.set_query_timeout(Duration::from_secs(5));
    kad::Behaviour::with_config(peer_id, store, config)
}

/// Test data helpers
pub mod test_data {
    use super::*;
    
    pub const SGF_TEST_DATA: &str = r#"(;FF[4]
CA[UTF-8]
GM[1]
DT[2025-06-29]
PC[OGS: https://online-go.com/game/76794817]
GN[Test Game]
PB[Player Black]
PW[Player White]
BR[7k]
WR[4k]
SZ[9]
KM[7.5]
RE[W+34.5]
;B[dc];W[ff];B[dg];W[ce];B[fh];W[fc];B[ec];W[fd];B[hg];W[bc])"#;
    
    pub fn create_test_position() -> Board {
        let mut board = Board::new(9);
        board.place_stone(Point::new(3, 2), Color::Black).unwrap();
        board.place_stone(Point::new(5, 5), Color::White).unwrap();
        board.place_stone(Point::new(3, 6), Color::Black).unwrap();
        board.place_stone(Point::new(2, 4), Color::White).unwrap();
        board
    }
    
    pub fn parse_sgf_to_game_state(rna: &RNAMessage) -> Option<Vec<Move>> {
        if let RNAType::SGFData { ref sgf_content, move_range, .. } = rna.rna_type {
            let game = parse_sgf(sgf_content).ok()?;
            let moves: Vec<Move> = game.main_variation()
                .skip(move_range.0)
                .take(move_range.1 - move_range.0)
                .filter_map(|node| {
                    node.get_move().map(|(color, point)| Move {
                        color,
                        point: Some(point),
                    })
                })
                .collect();
            Some(moves)
        } else {
            None
        }
    }
    
    pub fn evaluate_game_quality(moves: &[Move]) -> f32 {
        // Simple quality metric based on move distribution
        if moves.is_empty() {
            return 0.0;
        }
        
        let board_size = 9;
        let center = board_size / 2;
        let mut center_moves = 0;
        let mut corner_moves = 0;
        
        for m in moves.iter() {
            if let Some(point) = m.point {
                let dist_from_center = ((point.x as i32 - center).abs() + (point.y as i32 - center).abs()) as f32;
                if dist_from_center <= 2.0 {
                    center_moves += 1;
                } else if point.x <= 2 || point.x >= board_size - 3 || 
                         point.y <= 2 || point.y >= board_size - 3 {
                    corner_moves += 1;
                }
            }
        }
        
        // Good games have balanced play
        let balance = (center_moves as f32) / (moves.len() as f32);
        0.5 + (0.5 * balance.min(0.6))
    }
}

/// UUID generation helper
mod uuid {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    pub struct Uuid;
    
    impl Uuid {
        pub fn new_v4() -> String {
            format!("test-{}", COUNTER.fetch_add(1, Ordering::SeqCst))
        }
    }
}