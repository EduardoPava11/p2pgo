//! Game discovery service for P2P Go
//!
//! Provides game discovery through multiple mechanisms:
//! - Local mDNS for LAN games
//! - DHT queries for global games
//! - Relay-based discovery

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use anyhow::Result;
use libp2p::PeerId;
use tracing::{info, debug, warn};

use crate::p2p_node::{P2PNode, P2PEvent, GameMetadata};

/// Discovered game information
#[derive(Debug, Clone)]
pub struct DiscoveredGame {
    pub id: String,
    pub host_peer: PeerId,
    pub metadata: GameMetadata,
    pub discovery_source: DiscoverySource,
    pub connection_quality: ConnectionQuality,
    pub last_seen: std::time::Instant,
}

/// How the game was discovered
#[derive(Debug, Clone, PartialEq)]
pub enum DiscoverySource {
    /// Found via local mDNS
    Local,
    /// Found via DHT query
    DHT,
    /// Found via relay server
    Relay(PeerId),
    /// Directly shared (via ticket/invite)
    Direct,
}

/// Connection quality estimate
#[derive(Debug, Clone)]
pub struct ConnectionQuality {
    /// Can connect directly
    pub direct_possible: bool,
    /// Relay required
    pub relay_required: bool,
    /// Estimated latency (if known)
    pub latency_ms: Option<u32>,
}

/// Game discovery service
pub struct GameDiscovery {
    /// Discovered games
    games: Arc<RwLock<HashMap<String, DiscoveredGame>>>,
    /// P2P event receiver
    event_rx: mpsc::UnboundedReceiver<P2PEvent>,
    /// Discovery event sender
    discovery_tx: mpsc::UnboundedSender<DiscoveryEvent>,
    /// Connected peers info
    peer_info: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
}

/// Peer information for connection quality
#[derive(Debug, Clone)]
struct PeerInfo {
    peer_id: PeerId,
    addresses: Vec<libp2p::Multiaddr>,
    supports_relay: bool,
    last_ping: Option<Duration>,
}

/// Discovery events sent to UI
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// New game discovered
    GameDiscovered(DiscoveredGame),
    /// Game updated
    GameUpdated(DiscoveredGame),
    /// Game no longer available
    GameRemoved(String),
    /// Discovery statistics
    DiscoveryStats {
        total_games: usize,
        local_games: usize,
        relay_games: usize,
    },
}

use std::time::Duration;

impl GameDiscovery {
    /// Create a new game discovery service
    pub fn new(
        event_rx: mpsc::UnboundedReceiver<P2PEvent>,
        discovery_tx: mpsc::UnboundedSender<DiscoveryEvent>,
    ) -> Self {
        Self {
            games: Arc::new(RwLock::new(HashMap::new())),
            event_rx,
            discovery_tx,
            peer_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start the discovery service
    pub async fn start(mut self) -> Result<()> {
        info!("Starting game discovery service");
        
        // Start cleanup task
        let games = self.games.clone();
        let discovery_tx = self.discovery_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                cleanup_stale_games(&games, &discovery_tx).await;
            }
        });
        
        // Process P2P events
        while let Some(event) = self.event_rx.recv().await {
            if let Err(e) = self.handle_p2p_event(event).await {
                warn!("Error handling P2P event: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Handle P2P events
    async fn handle_p2p_event(&mut self, event: P2PEvent) -> Result<()> {
        match event {
            P2PEvent::GossipReceived { topic, data } => {
                if topic.contains("games") {
                    self.handle_game_announcement(data).await?;
                }
            }
            P2PEvent::DhtQueryComplete { key, providers } => {
                if String::from_utf8_lossy(&key).contains("games") {
                    self.handle_dht_results(providers).await?;
                }
            }
            P2PEvent::PeerConnected { peer_id } => {
                self.handle_peer_connected(peer_id).await?;
            }
            P2PEvent::PeerDisconnected { peer_id } => {
                self.handle_peer_disconnected(peer_id).await?;
            }
            P2PEvent::RelayDiscovered { peer_id, addresses } => {
                self.handle_relay_discovered(peer_id, addresses).await?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle game announcement via gossip
    async fn handle_game_announcement(&mut self, data: Vec<u8>) -> Result<()> {
        match serde_json::from_slice::<GameAnnouncement>(&data) {
            Ok(announcement) => {
                debug!("Received game announcement: {}", announcement.game_id);
                
                // Determine discovery source
                let source = if announcement.relay_peer.is_some() {
                    DiscoverySource::Relay(announcement.relay_peer.unwrap())
                } else {
                    DiscoverySource::DHT
                };
                
                // Estimate connection quality
                let quality = self.estimate_connection_quality(&announcement.host_peer).await;
                
                let game = DiscoveredGame {
                    id: announcement.game_id.clone(),
                    host_peer: announcement.host_peer,
                    metadata: announcement.metadata,
                    discovery_source: source,
                    connection_quality: quality,
                    last_seen: std::time::Instant::now(),
                };
                
                // Store game
                let mut games = self.games.write().await;
                let is_new = !games.contains_key(&game.id);
                games.insert(game.id.clone(), game.clone());
                
                // Send event
                if is_new {
                    self.discovery_tx.send(DiscoveryEvent::GameDiscovered(game))?;
                } else {
                    self.discovery_tx.send(DiscoveryEvent::GameUpdated(game))?;
                }
                
                // Update stats
                self.send_discovery_stats().await?;
            }
            Err(e) => {
                warn!("Failed to parse game announcement: {}", e);
            }
        }
        Ok(())
    }
    
    /// Handle DHT query results
    async fn handle_dht_results(&mut self, providers: Vec<PeerId>) -> Result<()> {
        debug!("DHT query returned {} providers", providers.len());
        
        for peer_id in providers {
            // Query each provider for their games
            // This would trigger additional P2P communication
            info!("Found game provider: {}", peer_id);
        }
        
        Ok(())
    }
    
    /// Handle peer connection
    async fn handle_peer_connected(&mut self, peer_id: PeerId) -> Result<()> {
        debug!("Peer connected: {}", peer_id);
        
        // Update peer info
        let mut peer_info = self.peer_info.write().await;
        peer_info.insert(peer_id, PeerInfo {
            peer_id,
            addresses: vec![],
            supports_relay: false,
            last_ping: None,
        });
        
        // Check if this peer has any games
        // In a real implementation, we'd query the peer
        
        Ok(())
    }
    
    /// Handle peer disconnection
    async fn handle_peer_disconnected(&mut self, peer_id: PeerId) -> Result<()> {
        debug!("Peer disconnected: {}", peer_id);
        
        // Remove peer info
        self.peer_info.write().await.remove(&peer_id);
        
        // Remove any games hosted by this peer
        let mut games = self.games.write().await;
        let removed: Vec<_> = games.iter()
            .filter(|(_, game)| game.host_peer == peer_id)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in removed {
            games.remove(&id);
            self.discovery_tx.send(DiscoveryEvent::GameRemoved(id))?;
        }
        
        self.send_discovery_stats().await?;
        
        Ok(())
    }
    
    /// Handle relay discovery
    async fn handle_relay_discovered(&mut self, peer_id: PeerId, addresses: Vec<libp2p::Multiaddr>) -> Result<()> {
        debug!("Relay discovered: {} with {} addresses", peer_id, addresses.len());
        
        // Update peer info
        let mut peer_info = self.peer_info.write().await;
        if let Some(info) = peer_info.get_mut(&peer_id) {
            info.supports_relay = true;
            info.addresses = addresses;
        }
        
        Ok(())
    }
    
    /// Estimate connection quality to a peer
    async fn estimate_connection_quality(&self, peer_id: &PeerId) -> ConnectionQuality {
        let peer_info = self.peer_info.read().await;
        
        if let Some(info) = peer_info.get(peer_id) {
            ConnectionQuality {
                direct_possible: !info.addresses.is_empty(),
                relay_required: info.addresses.is_empty(),
                latency_ms: info.last_ping.map(|d| d.as_millis() as u32),
            }
        } else {
            // Unknown peer, assume relay required
            ConnectionQuality {
                direct_possible: false,
                relay_required: true,
                latency_ms: None,
            }
        }
    }
    
    /// Send discovery statistics
    async fn send_discovery_stats(&self) -> Result<()> {
        let games = self.games.read().await;
        
        let local_games = games.values()
            .filter(|g| g.discovery_source == DiscoverySource::Local)
            .count();
            
        let relay_games = games.values()
            .filter(|g| matches!(g.discovery_source, DiscoverySource::Relay(_)))
            .count();
        
        self.discovery_tx.send(DiscoveryEvent::DiscoveryStats {
            total_games: games.len(),
            local_games,
            relay_games,
        })?;
        
        Ok(())
    }
    
    /// Get all discovered games
    pub async fn get_games(&self) -> Vec<DiscoveredGame> {
        self.games.read().await.values().cloned().collect()
    }
    
    /// Get games filtered by board size
    pub async fn get_games_by_size(&self, board_size: u8) -> Vec<DiscoveredGame> {
        self.games.read().await
            .values()
            .filter(|g| g.metadata.board_size == board_size)
            .cloned()
            .collect()
    }
    
    /// Manually add a game (e.g., from direct invite)
    pub async fn add_game_direct(&self, peer_id: PeerId, game_id: String, metadata: GameMetadata) -> Result<()> {
        let game = DiscoveredGame {
            id: game_id.clone(),
            host_peer: peer_id,
            metadata,
            discovery_source: DiscoverySource::Direct,
            connection_quality: ConnectionQuality {
                direct_possible: true,
                relay_required: false,
                latency_ms: None,
            },
            last_seen: std::time::Instant::now(),
        };
        
        self.games.write().await.insert(game_id, game.clone());
        self.discovery_tx.send(DiscoveryEvent::GameDiscovered(game))?;
        
        Ok(())
    }
}

/// Clean up stale games
async fn cleanup_stale_games(
    games: &Arc<RwLock<HashMap<String, DiscoveredGame>>>,
    discovery_tx: &mpsc::UnboundedSender<DiscoveryEvent>,
) {
    let mut games = games.write().await;
    let now = std::time::Instant::now();
    let stale_timeout = Duration::from_secs(120); // 2 minutes
    
    let stale: Vec<_> = games.iter()
        .filter(|(_, game)| now.duration_since(game.last_seen) > stale_timeout)
        .map(|(id, _)| id.clone())
        .collect();
    
    for id in stale {
        games.remove(&id);
        let _ = discovery_tx.send(DiscoveryEvent::GameRemoved(id));
    }
}

/// Game announcement format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GameAnnouncement {
    pub game_id: String,
    pub host_peer: PeerId,
    pub metadata: GameMetadata,
    pub relay_peer: Option<PeerId>,
}