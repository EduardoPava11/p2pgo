//! P2P Go Relay Server
//!
//! A decentralized relay server that:
//! - Hosts game data for the network
//! - Connects to other relays via gossip protocol
//! - Provides shortest path routing for game observers
//! - Supports bandwidth throttling and QoS

use anyhow::{Result, Context};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use serde::{Serialize, Deserialize};

use crate::relay_mesh::{RelayMesh, RelayNode, RelayCapabilities, RelayStats, GossipConfig};
use crate::relay_monitor::RelayMonitor;
use crate::port::pick_available_port;

/// Relay server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayServerConfig {
    /// Bind address for the relay
    pub bind_address: String,
    /// Port to listen on (0 for auto)
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Maximum bandwidth in Mbps
    pub max_bandwidth_mbps: f64,
    /// Whether to store game history
    pub store_history: bool,
    /// Bootstrap relay nodes to connect to
    pub bootstrap_relays: Vec<String>,
    /// Gossip protocol settings
    pub gossip: GossipConfig,
    /// Enable relay mesh networking
    pub enable_mesh: bool,
}

impl Default for RelayServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 0, // Auto-select
            max_connections: 1000,
            max_bandwidth_mbps: 100.0,
            store_history: true,
            bootstrap_relays: vec![],
            gossip: GossipConfig::default(),
            enable_mesh: true,
        }
    }
}

/// P2P Go Relay Server
pub struct RelayServer {
    /// Server configuration
    config: RelayServerConfig,
    /// Our relay node info
    node: RelayNode,
    /// Relay mesh network
    mesh: Option<Arc<RelayMesh>>,
    /// Health monitor
    monitor: Arc<RelayMonitor>,
    /// Game storage
    game_store: Arc<RwLock<GameStore>>,
    /// Active connections
    connections: Arc<RwLock<ConnectionManager>>,
}

/// Game data storage
#[derive(Default)]
struct GameStore {
    /// Active games by ID
    games: std::collections::HashMap<String, GameData>,
    /// Game metadata index
    index: GameIndex,
}

/// Game metadata for quick lookups
#[derive(Default)]
struct GameIndex {
    /// Games by board size
    by_board_size: std::collections::HashMap<u8, Vec<String>>,
    /// Games by player
    by_player: std::collections::HashMap<String, Vec<String>>,
    /// Recently active games
    recent: std::collections::VecDeque<(String, std::time::Instant)>,
}

/// Stored game data
#[derive(Clone)]
pub struct GameData {
    /// Game ID
    id: String,
    /// Board size
    board_size: u8,
    /// Player IDs
    players: (String, String),
    /// Compressed move data
    _moves: Vec<u8>,
    /// Last update time
    last_update: std::time::Instant,
    /// Observers currently watching
    _observers: std::collections::HashSet<String>,
}

/// Connection management
#[derive(Default)]
struct ConnectionManager {
    /// Active connections by peer ID
    connections: std::collections::HashMap<String, ConnectionInfo>,
    /// Bandwidth tracking
    bandwidth_tracker: BandwidthTracker,
}

/// Connection information
struct ConnectionInfo {
    /// Peer ID
    _peer_id: String,
    /// Connection address
    _address: SocketAddr,
    /// Connection start time
    _connected_at: std::time::Instant,
    /// Bandwidth usage
    _bandwidth_usage: f64,
    /// Games being observed
    _observing: std::collections::HashSet<String>,
}

/// Bandwidth tracking and QoS
#[derive(Default)]
struct BandwidthTracker {
    /// Total bandwidth usage
    total_usage_mbps: f64,
    /// Per-connection limits
    _connection_limits: std::collections::HashMap<String, f64>,
    /// Global limit
    _global_limit_mbps: f64,
}

impl RelayServer {
    /// Create a new relay server
    pub async fn new(config: RelayServerConfig) -> Result<Self> {
        // Auto-select port if needed
        let port = if config.port == 0 {
            pick_available_port()?
        } else {
            config.port
        };
        
        let bind_addr = format!("{}:{}", config.bind_address, port);
        let socket_addr: SocketAddr = bind_addr.parse()
            .context("Invalid bind address")?;
        
        // Create node info
        let node = RelayNode {
            id: generate_node_id(),
            address: socket_addr,
            capabilities: RelayCapabilities {
                max_bandwidth_mbps: config.max_bandwidth_mbps,
                max_connections: config.max_connections,
                board_sizes: vec![9, 13, 19],
                stores_history: config.store_history,
                supports_observers: true,
            },
            last_seen: std::time::Instant::now(),
            stats: RelayStats::default(),
        };
        
        // Create relay mesh if enabled
        let mesh = if config.enable_mesh {
            Some(RelayMesh::new(node.clone(), config.gossip.clone()))
        } else {
            None
        };
        
        // Create health monitor
        let monitor = Arc::new(RelayMonitor::new_stub(vec![socket_addr.to_string()]));
        
        Ok(Self {
            config,
            node,
            mesh,
            monitor,
            game_store: Arc::new(RwLock::new(GameStore::default())),
            connections: Arc::new(RwLock::new(ConnectionManager::default())),
        })
    }
    
    /// Start the relay server
    pub async fn start(&self) -> Result<()> {
        info!("Starting P2P Go Relay Server on {}", self.node.address);
        
        // Start relay mesh if enabled
        if let Some(mesh) = &self.mesh {
            info!("Starting relay mesh networking");
            let mesh_clone = mesh.clone();
            tokio::spawn(async move {
                mesh_clone.start_gossip().await;
            });
            
            // Connect to bootstrap relays
            for relay_addr in &self.config.bootstrap_relays {
                info!("Connecting to bootstrap relay: {}", relay_addr);
                // TODO: Implement bootstrap connection
            }
        }
        
        // Health status would be updated here in real implementation
        
        // Start connection handler
        self.start_connection_handler().await?;
        
        // Start game cleanup task
        self.start_game_cleanup_task();
        
        // Start metrics reporter
        self.start_metrics_reporter();
        
        info!("Relay server started successfully");
        Ok(())
    }
    
    /// Handle incoming connections
    async fn start_connection_handler(&self) -> Result<()> {
        // TODO: Implement actual network listener
        // For now, this is a placeholder
        
        let connections = self.connections.clone();
        let _game_store = self.game_store.clone();
        let _monitor = self.monitor.clone();
        
        tokio::spawn(async move {
            info!("Connection handler started");
            
            // Simulate connection handling
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                
                // Update connection stats
                let conn_count = connections.read().await.connections.len();
                info!("Active connections: {}", conn_count);
            }
        });
        
        Ok(())
    }
    
    /// Clean up old game data
    fn start_game_cleanup_task(&self) {
        let game_store = self.game_store.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
            
            loop {
                interval.tick().await;
                
                let now = std::time::Instant::now();
                let mut store = game_store.write().await;
                
                // Remove games older than 24 hours
                store.games.retain(|id, game| {
                    let age = now.duration_since(game.last_update);
                    if age > std::time::Duration::from_secs(86400) {
                        info!("Removing old game: {}", id);
                        false
                    } else {
                        true
                    }
                });
                
                // Rebuild index
                let games_snapshot: Vec<_> = store.games.iter()
                    .map(|(id, game)| (id.clone(), game.clone()))
                    .collect();
                
                store.index.by_board_size.clear();
                store.index.by_player.clear();
                
                for (id, game) in games_snapshot {
                    store.index.by_board_size
                        .entry(game.board_size)
                        .or_insert_with(Vec::new)
                        .push(id.clone());
                    
                    store.index.by_player
                        .entry(game.players.0.clone())
                        .or_insert_with(Vec::new)
                        .push(id.clone());
                    
                    store.index.by_player
                        .entry(game.players.1.clone())
                        .or_insert_with(Vec::new)
                        .push(id.clone());
                }
            }
        });
    }
    
    /// Report metrics periodically
    fn start_metrics_reporter(&self) {
        let connections = self.connections.clone();
        let game_store = self.game_store.clone();
        let _node = self.node.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let conn_mgr = connections.read().await;
                let store = game_store.read().await;
                
                info!(
                    "Relay metrics - Connections: {}, Games: {}, Bandwidth: {:.2} Mbps",
                    conn_mgr.connections.len(),
                    store.games.len(),
                    conn_mgr.bandwidth_tracker.total_usage_mbps
                );
                
                // Log per-board-size stats
                for (size, games) in &store.index.by_board_size {
                    info!("  {}x{} games: {}", size, size, games.len());
                }
            }
        });
    }
    
    /// Store game data
    pub async fn store_game(&self, game_data: GameData) -> Result<()> {
        let mut store = self.game_store.write().await;
        
        info!("Storing game {} ({}x{})", game_data.id, game_data.board_size, game_data.board_size);
        
        // Update index
        store.index.by_board_size
            .entry(game_data.board_size)
            .or_insert_with(Vec::new)
            .push(game_data.id.clone());
        
        store.index.by_player
            .entry(game_data.players.0.clone())
            .or_insert_with(Vec::new)
            .push(game_data.id.clone());
        
        store.index.by_player
            .entry(game_data.players.1.clone())
            .or_insert_with(Vec::new)
            .push(game_data.id.clone());
        
        // Add to recent games
        store.index.recent.push_back((game_data.id.clone(), std::time::Instant::now()));
        if store.index.recent.len() > 100 {
            store.index.recent.pop_front();
        }
        
        // Store game
        store.games.insert(game_data.id.clone(), game_data);
        
        Ok(())
    }
    
    /// Get game data
    pub async fn get_game(&self, game_id: &str) -> Option<GameData> {
        let store = self.game_store.read().await;
        store.games.get(game_id).cloned()
    }
    
    /// Find games by criteria
    pub async fn find_games(&self, board_size: Option<u8>, player: Option<&str>) -> Vec<String> {
        let store = self.game_store.read().await;
        
        if let Some(size) = board_size {
            store.index.by_board_size.get(&size)
                .map(|games| games.clone())
                .unwrap_or_default()
        } else if let Some(player_id) = player {
            store.index.by_player.get(player_id)
                .map(|games| games.clone())
                .unwrap_or_default()
        } else {
            // Return recent games
            store.index.recent.iter()
                .map(|(id, _)| id.clone())
                .collect()
        }
    }
    
    /// Get relay statistics
    pub async fn get_stats(&self) -> RelayStats {
        let conn_mgr = self.connections.read().await;
        let store = self.game_store.read().await;
        
        RelayStats {
            current_bandwidth_mbps: conn_mgr.bandwidth_tracker.total_usage_mbps,
            active_connections: conn_mgr.connections.len(),
            active_games: store.games.len(),
            total_games_served: store.games.len() as u64, // TODO: Track total
            avg_latency_ms: 0.0, // TODO: Measure
            packet_loss_rate: 0.0, // TODO: Measure
        }
    }
    
    /// Shutdown the relay server
    pub async fn shutdown(&self) {
        info!("Shutting down relay server");
        
        // Close all connections
        let mut conn_mgr = self.connections.write().await;
        conn_mgr.connections.clear();
        
        info!("Relay server stopped");
    }
}

/// Generate a unique node ID
fn generate_node_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

/// Relay server builder for easy configuration
pub struct RelayServerBuilder {
    config: RelayServerConfig,
}

impl RelayServerBuilder {
    pub fn new() -> Self {
        Self {
            config: RelayServerConfig::default(),
        }
    }
    
    pub fn bind_address(mut self, addr: &str) -> Self {
        self.config.bind_address = addr.to_string();
        self
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }
    
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }
    
    pub fn max_bandwidth_mbps(mut self, mbps: f64) -> Self {
        self.config.max_bandwidth_mbps = mbps;
        self
    }
    
    pub fn bootstrap_relays(mut self, relays: Vec<String>) -> Self {
        self.config.bootstrap_relays = relays;
        self
    }
    
    pub fn enable_mesh(mut self, enable: bool) -> Self {
        self.config.enable_mesh = enable;
        self
    }
    
    pub fn gossip_config(mut self, config: GossipConfig) -> Self {
        self.config.gossip = config;
        self
    }
    
    pub async fn build(self) -> Result<RelayServer> {
        RelayServer::new(self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_relay_server_creation() {
        let server = RelayServerBuilder::new()
            .bind_address("127.0.0.1")
            .max_connections(100)
            .enable_mesh(false)
            .build()
            .await
            .unwrap();
        
        assert_eq!(server.config.max_connections, 100);
        assert!(server.mesh.is_none());
    }
}