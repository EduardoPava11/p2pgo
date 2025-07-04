//! Relay Mesh Network Protocol
//!
//! This module implements a decentralized relay mesh network where:
//! - Relays can discover and connect to each other
//! - Game data is gossiped between relays with configurable intervals
//! - Shortest path routing enables efficient game observation
//! - Bandwidth throttling controls network usage

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};

/// Relay node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayNode {
    /// Unique relay ID (public key)
    pub id: String,
    /// Network address
    pub address: SocketAddr,
    /// Supported capabilities
    pub capabilities: RelayCapabilities,
    /// Last seen timestamp (as unix timestamp)
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    pub last_seen: Instant,
    /// Network statistics
    pub stats: RelayStats,
}

/// Relay capabilities and features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayCapabilities {
    /// Maximum bandwidth in Mbps
    pub max_bandwidth_mbps: f64,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Supported board sizes
    pub board_sizes: Vec<u8>,
    /// Whether this relay stores game history
    pub stores_history: bool,
    /// Whether this relay supports observer mode
    pub supports_observers: bool,
}

/// Network statistics for a relay
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelayStats {
    /// Current bandwidth usage in Mbps
    pub current_bandwidth_mbps: f64,
    /// Active connections
    pub active_connections: usize,
    /// Games being relayed
    pub active_games: usize,
    /// Total games served
    pub total_games_served: u64,
    /// Average latency to this relay
    pub avg_latency_ms: f64,
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss_rate: f64,
}

/// Gossip message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// Announce relay presence
    RelayAnnounce {
        node: RelayNode,
        /// Known peers for mesh discovery
        known_peers: Vec<String>,
    },
    /// Share game metadata
    GameAvailable {
        game_id: String,
        board_size: u8,
        players: (String, String),
        move_count: u32,
        /// Relays that have this game data
        available_at: Vec<String>,
    },
    /// Request game data
    GameRequest {
        game_id: String,
        /// Moves already known (for incremental updates)
        known_moves: u32,
    },
    /// Game data response
    GameData {
        game_id: String,
        /// Compressed move data
        move_data: Vec<u8>,
        /// Starting from move number
        from_move: u32,
        /// Total moves in game
        total_moves: u32,
    },
    /// Network topology update
    TopologyUpdate {
        /// Relay connectivity graph
        edges: Vec<(String, String, f64)>, // (from, to, latency_ms)
    },
    /// Bandwidth throttle request
    ThrottleRequest {
        relay_id: String,
        /// Requested bandwidth limit in Mbps
        limit_mbps: f64,
    },
}

/// Gossip protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    /// How often to announce presence (in seconds)
    #[serde(with = "humantime_serde")]
    pub announce_interval: Duration,
    /// How often to share game metadata (in seconds)
    #[serde(with = "humantime_serde")]
    pub game_share_interval: Duration,
    /// How often to update topology (in seconds)
    #[serde(with = "humantime_serde")]
    pub topology_interval: Duration,
    /// Maximum peers to gossip with per round
    pub gossip_fanout: usize,
    /// Peer timeout before removal (in seconds)
    #[serde(with = "humantime_serde")]
    pub peer_timeout: Duration,
    /// Maximum bandwidth per peer in Mbps
    pub max_peer_bandwidth_mbps: f64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            announce_interval: Duration::from_secs(30),
            game_share_interval: Duration::from_secs(60),
            topology_interval: Duration::from_secs(120),
            gossip_fanout: 5,
            peer_timeout: Duration::from_secs(300),
            max_peer_bandwidth_mbps: 10.0,
        }
    }
}

/// Relay mesh network manager
pub struct RelayMesh {
    /// Our relay node info
    local_node: RelayNode,
    /// Known relay peers
    peers: Arc<RwLock<HashMap<String, RelayNode>>>,
    /// Network topology for routing
    topology: Arc<RwLock<NetworkTopology>>,
    /// Gossip configuration
    config: GossipConfig,
    /// Message send channel
    tx: mpsc::Sender<(String, GossipMessage)>,
    /// Message receive channel
    rx: Arc<RwLock<mpsc::Receiver<(String, GossipMessage)>>>,
}

/// Network topology for shortest path routing
#[derive(Debug)]
struct NetworkTopology {
    /// Adjacency list with latencies
    edges: HashMap<String, Vec<(String, f64)>>,
    /// Cached shortest paths
    path_cache: HashMap<(String, String), Vec<String>>,
    /// Last update time
    last_update: Instant,
}

impl Default for NetworkTopology {
    fn default() -> Self {
        Self {
            edges: HashMap::new(),
            path_cache: HashMap::new(),
            last_update: Instant::now(),
        }
    }
}

impl NetworkTopology {
    /// Find shortest path between two relays using Dijkstra's algorithm
    fn _shortest_path(&mut self, from: &str, to: &str) -> Option<Vec<String>> {
        // Check cache first
        let cache_key = (from.to_string(), to.to_string());
        if let Some(path) = self.path_cache.get(&cache_key) {
            return Some(path.clone());
        }
        
        // Dijkstra's algorithm
        let mut distances: HashMap<String, f64> = HashMap::new();
        let mut previous: HashMap<String, String> = HashMap::new();
        let mut unvisited: HashSet<String> = self.edges.keys().cloned().collect();
        
        distances.insert(from.to_string(), 0.0);
        
        while !unvisited.is_empty() {
            // Find unvisited node with minimum distance
            let current = unvisited.iter()
                .min_by(|a, b| {
                    let dist_a = distances.get(*a).unwrap_or(&f64::INFINITY);
                    let dist_b = distances.get(*b).unwrap_or(&f64::INFINITY);
                    dist_a.partial_cmp(dist_b).unwrap()
                })
                .cloned()?;
            
            if &current == to {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = to.to_string();
                
                while node != from {
                    path.push(node.clone());
                    node = previous.get(&node)?.clone();
                }
                path.push(from.to_string());
                path.reverse();
                
                // Cache the result
                self.path_cache.insert(cache_key, path.clone());
                return Some(path);
            }
            
            unvisited.remove(&current);
            
            let current_dist = *distances.get(&current).unwrap_or(&f64::INFINITY);
            if current_dist == f64::INFINITY {
                break;
            }
            
            // Update distances to neighbors
            if let Some(neighbors) = self.edges.get(&current) {
                for (neighbor, latency) in neighbors {
                    if unvisited.contains(neighbor) {
                        let alt_dist = current_dist + latency;
                        let neighbor_dist = distances.get(neighbor).unwrap_or(&f64::INFINITY);
                        
                        if alt_dist < *neighbor_dist {
                            distances.insert(neighbor.clone(), alt_dist);
                            previous.insert(neighbor.clone(), current.clone());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Update topology with new edge information
    fn update_edges(&mut self, edges: Vec<(String, String, f64)>) {
        // Clear cache on topology update
        self.path_cache.clear();
        self.edges.clear();
        
        for (from, to, latency) in edges {
            self.edges.entry(from.clone())
                .or_insert_with(Vec::new)
                .push((to.clone(), latency));
            
            // Add reverse edge for bidirectional connectivity
            self.edges.entry(to)
                .or_insert_with(Vec::new)
                .push((from, latency));
        }
        
        self.last_update = Instant::now();
    }
}

impl RelayMesh {
    /// Create a new relay mesh network
    pub fn new(local_node: RelayNode, config: GossipConfig) -> Arc<Self> {
        let (tx, rx) = mpsc::channel(1000);
        
        Arc::new(Self {
            local_node,
            peers: Arc::new(RwLock::new(HashMap::new())),
            topology: Arc::new(RwLock::new(NetworkTopology::default())),
            config,
            tx,
            rx: Arc::new(RwLock::new(rx)),
        })
    }
    
    /// Start the gossip protocol
    pub async fn start_gossip(&self) {
        // Start announce task
        let announce_task = self.start_announce_task();
        
        // Start game sharing task
        let game_share_task = self.start_game_share_task();
        
        // Start topology update task
        let topology_task = self.start_topology_task();
        
        // Start message handler
        let handler_task = self.start_message_handler();
        
        // Start peer cleanup task
        let cleanup_task = self.start_cleanup_task();
        
        // Run all tasks
        tokio::select! {
            _ = announce_task => warn!("Announce task ended"),
            _ = game_share_task => warn!("Game share task ended"),
            _ = topology_task => warn!("Topology task ended"),
            _ = handler_task => warn!("Message handler ended"),
            _ = cleanup_task => warn!("Cleanup task ended"),
        }
    }
    
    /// Send a gossip message to specific peers
    pub async fn gossip_to_peers(&self, message: GossipMessage, peer_ids: Vec<String>) {
        for peer_id in peer_ids {
            if let Err(e) = self.tx.send((peer_id, message.clone())).await {
                warn!("Failed to queue gossip message: {}", e);
            }
        }
    }
    
    /// Select random peers for gossiping
    async fn select_gossip_peers(&self, count: usize) -> Vec<String> {
        let peers = self.peers.read().await;
        let mut peer_ids: Vec<String> = peers.keys().cloned().collect();
        
        // Shuffle and take up to count peers
        use rand::seq::SliceRandom;
        peer_ids.shuffle(&mut rand::thread_rng());
        peer_ids.truncate(count);
        
        peer_ids
    }
    
    /// Start periodic relay announcement
    async fn start_announce_task(&self) -> tokio::task::JoinHandle<()> {
        let peers = self.peers.clone();
        let local_node = self.local_node.clone();
        let interval = self.config.announce_interval;
        let fanout = self.config.gossip_fanout;
        let mesh = self.clone_for_task();
        
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(interval);
            
            loop {
                timer.tick().await;
                
                // Get known peers for announcement
                let known_peers: Vec<String> = {
                    let peers_lock = peers.read().await;
                    peers_lock.keys().take(10).cloned().collect()
                };
                
                let message = GossipMessage::RelayAnnounce {
                    node: local_node.clone(),
                    known_peers,
                };
                
                let targets = mesh.select_gossip_peers(fanout).await;
                mesh.gossip_to_peers(message, targets).await;
                
                debug!("Announced relay presence");
            }
        })
    }
    
    /// Start periodic game metadata sharing
    async fn start_game_share_task(&self) -> tokio::task::JoinHandle<()> {
        let interval = self.config.game_share_interval;
        let fanout = self.config.gossip_fanout;
        let mesh = self.clone_for_task();
        
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(interval);
            
            loop {
                timer.tick().await;
                
                // TODO: Get active games from game manager
                let games = vec![]; // Placeholder
                
                for game in games {
                    let message = GossipMessage::GameAvailable {
                        game_id: game,
                        board_size: 9,
                        players: ("player1".to_string(), "player2".to_string()),
                        move_count: 0,
                        available_at: vec![mesh.local_node.id.clone()],
                    };
                    
                    let targets = mesh.select_gossip_peers(fanout).await;
                    mesh.gossip_to_peers(message, targets).await;
                }
                
                debug!("Shared game metadata");
            }
        })
    }
    
    /// Start periodic topology updates
    async fn start_topology_task(&self) -> tokio::task::JoinHandle<()> {
        let interval = self.config.topology_interval;
        let peers = self.peers.clone();
        let mesh = self.clone_for_task();
        
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(interval);
            
            loop {
                timer.tick().await;
                
                // Measure latencies to peers
                let mut edges = Vec::new();
                let peers_lock = peers.read().await;
                
                for (peer_id, peer_node) in peers_lock.iter() {
                    // TODO: Actual latency measurement
                    let latency = peer_node.stats.avg_latency_ms;
                    edges.push((
                        mesh.local_node.id.clone(),
                        peer_id.clone(),
                        latency
                    ));
                }
                
                let message = GossipMessage::TopologyUpdate { edges };
                let targets = mesh.select_gossip_peers(mesh.config.gossip_fanout).await;
                mesh.gossip_to_peers(message, targets).await;
                
                debug!("Shared topology update");
            }
        })
    }
    
    /// Handle incoming gossip messages
    async fn start_message_handler(&self) -> tokio::task::JoinHandle<()> {
        let rx = self.rx.clone();
        let peers = self.peers.clone();
        let topology = self.topology.clone();
        
        tokio::spawn(async move {
            let mut rx = rx.write().await;
            
            while let Some((from_peer, message)) = rx.recv().await {
                match message {
                    GossipMessage::RelayAnnounce { node, known_peers } => {
                        // Update peer information
                        peers.write().await.insert(node.id.clone(), node);
                        
                        // Learn about new peers
                        for peer_id in known_peers {
                            if !peers.read().await.contains_key(&peer_id) {
                                debug!("Discovered new peer: {}", peer_id);
                            }
                        }
                    }
                    
                    GossipMessage::GameAvailable { game_id, .. } => {
                        debug!("Game {} available", game_id);
                        // TODO: Update game directory
                    }
                    
                    GossipMessage::TopologyUpdate { edges } => {
                        topology.write().await.update_edges(edges);
                        debug!("Updated network topology");
                    }
                    
                    _ => {
                        debug!("Received gossip message from {}", from_peer);
                    }
                }
            }
        })
    }
    
    /// Clean up stale peers
    async fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let peers = self.peers.clone();
        let timeout = self.config.peer_timeout;
        
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                timer.tick().await;
                
                let now = Instant::now();
                let mut peers_lock = peers.write().await;
                
                peers_lock.retain(|id, node| {
                    let age = now.duration_since(node.last_seen);
                    if age > timeout {
                        info!("Removing stale peer: {}", id);
                        false
                    } else {
                        true
                    }
                });
            }
        })
    }
    
    /// Clone relay mesh for use in tasks
    fn clone_for_task(&self) -> Self {
        Self {
            local_node: self.local_node.clone(),
            peers: self.peers.clone(),
            topology: self.topology.clone(),
            config: self.config.clone(),
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
    
    /// Find best path to observe a game
    pub async fn find_game_route(&self, _game_id: &str) -> Option<Vec<String>> {
        // TODO: Implement game routing logic
        // For now, return None
        None
    }
    
    /// Get current bandwidth usage
    pub async fn get_bandwidth_usage(&self) -> BandwidthReport {
        let peers = self.peers.read().await;
        
        let mut total_bandwidth = 0.0;
        let mut peer_bandwidth = HashMap::new();
        
        for (peer_id, node) in peers.iter() {
            let bandwidth = node.stats.current_bandwidth_mbps;
            total_bandwidth += bandwidth;
            peer_bandwidth.insert(peer_id.clone(), bandwidth);
        }
        
        BandwidthReport {
            total_bandwidth_mbps: total_bandwidth,
            peer_bandwidth_mbps: peer_bandwidth,
            limit_mbps: self.config.max_peer_bandwidth_mbps * peers.len() as f64,
        }
    }
    
    /// Throttle bandwidth to specific peer
    pub async fn throttle_peer(&self, peer_id: &str, limit_mbps: f64) {
        let message = GossipMessage::ThrottleRequest {
            relay_id: self.local_node.id.clone(),
            limit_mbps,
        };
        
        self.gossip_to_peers(message, vec![peer_id.to_string()]).await;
        info!("Requested bandwidth throttle to {} for peer {}", limit_mbps, peer_id);
    }
}

/// Bandwidth usage report
#[derive(Debug, Clone)]
pub struct BandwidthReport {
    /// Total bandwidth being used
    pub total_bandwidth_mbps: f64,
    /// Per-peer bandwidth usage
    pub peer_bandwidth_mbps: HashMap<String, f64>,
    /// Current bandwidth limit
    pub limit_mbps: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_topology() {
        let mut topology = NetworkTopology::default();
        
        // Create a simple network: A -> B -> C
        topology.update_edges(vec![
            ("A".to_string(), "B".to_string(), 10.0),
            ("B".to_string(), "C".to_string(), 20.0),
            ("A".to_string(), "C".to_string(), 50.0), // Direct but slower
        ]);
        
        // Shortest path should be A -> B -> C (total: 30ms)
        let path = topology.shortest_path("A", "C").unwrap();
        assert_eq!(path, vec!["A", "B", "C"]);
    }
    
    #[test]
    fn test_gossip_config() {
        let config = GossipConfig::default();
        assert_eq!(config.gossip_fanout, 5);
        assert_eq!(config.announce_interval, Duration::from_secs(30));
    }
}