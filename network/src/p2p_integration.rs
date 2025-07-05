//! Integration layer between P2P node and game logic
//!
//! This module connects the P2P networking layer with the game channels,
//! providing persistent connections and automatic reconnection.

use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use anyhow::{Result, anyhow};
use libp2p::{PeerId, Multiaddr};
use tracing::{info, warn, error, debug};
use std::collections::HashMap;

use crate::{
    p2p_node::{P2PNode, P2PConfig, RelayMode, P2PEvent, GameMetadata},
    game_discovery::{GameDiscovery, DiscoveryEvent},
    game_channel::GameChannel,
    GameId,
};

/// P2P integration manager
pub struct P2PIntegration {
    /// The P2P node
    node: Arc<RwLock<P2PNode>>,
    /// Game discovery service
    discovery: Arc<GameDiscovery>,
    /// Active game sessions
    game_sessions: Arc<RwLock<HashMap<GameId, GameSession>>>,
    /// Event receivers
    p2p_event_rx: mpsc::UnboundedReceiver<P2PEvent>,
    discovery_event_rx: mpsc::UnboundedReceiver<DiscoveryEvent>,
    /// UI event sender
    ui_event_tx: mpsc::UnboundedSender<P2PIntegrationEvent>,
}

/// Active game session with persistent connection
struct GameSession {
    game_id: GameId,
    channel: Arc<GameChannel>,
    remote_peer: PeerId,
    connection_state: ConnectionState,
    reconnect_attempts: u32,
}

/// Connection state for a game session
#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
    Failed,
}

/// Events sent to UI
#[derive(Debug, Clone)]
pub enum P2PIntegrationEvent {
    /// Game discovered
    GameDiscovered {
        game_id: String,
        host_name: String,
        board_size: u8,
        requires_relay: bool,
    },
    /// Connected to game
    GameConnected {
        game_id: String,
        peer_id: PeerId,
    },
    /// Disconnected from game
    GameDisconnected {
        game_id: String,
        reason: String,
    },
    /// Reconnecting to game
    GameReconnecting {
        game_id: String,
        attempt: u32,
    },
    /// Connection statistics
    ConnectionStats {
        connected_peers: usize,
        active_games: usize,
        relay_active: bool,
    },
}

impl P2PIntegration {
    /// Create a new P2P integration
    pub async fn new(
        keypair: libp2p::identity::Keypair,
        config: P2PConfig,
        ui_event_tx: mpsc::UnboundedSender<P2PIntegrationEvent>,
    ) -> Result<Self> {
        // Create channels
        let (p2p_tx, p2p_rx) = mpsc::unbounded_channel();
        let (discovery_tx, discovery_rx) = mpsc::unbounded_channel();
        
        // Create P2P node
        let node = P2PNode::new(keypair, config, p2p_tx).await?;
        let node = Arc::new(RwLock::new(node));
        
        // Create discovery service
        let (discovery_p2p_tx, discovery_p2p_rx) = mpsc::unbounded_channel();
        let discovery = Arc::new(GameDiscovery::new(discovery_p2p_rx, discovery_tx));
        
        Ok(Self {
            node,
            discovery,
            game_sessions: Arc::new(RwLock::new(HashMap::new())),
            p2p_event_rx: p2p_rx,
            discovery_event_rx: discovery_rx,
            ui_event_tx,
        })
    }
    
    /// Start the P2P integration
    pub async fn start(mut self) -> Result<()> {
        info!("Starting P2P integration");
        
        // Start P2P node
        {
            let mut node = self.node.write().await;
            node.start().await?;
        }
        
        // Start discovery service
        let discovery = self.discovery.clone();
        tokio::spawn(async move {
            if let Err(e) = discovery.start().await {
                error!("Discovery service error: {}", e);
            }
        });
        
        // Start reconnection manager
        let sessions = self.game_sessions.clone();
        let node = self.node.clone();
        tokio::spawn(async move {
            reconnection_manager(sessions, node).await;
        });
        
        // Handle events
        self.run_event_loop().await
    }
    
    /// Main event loop
    async fn run_event_loop(mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(event) = self.p2p_event_rx.recv() => {
                    self.handle_p2p_event(event).await?;
                }
                Some(event) = self.discovery_event_rx.recv() => {
                    self.handle_discovery_event(event).await?;
                }
            }
        }
    }
    
    /// Handle P2P events
    async fn handle_p2p_event(&mut self, event: P2PEvent) -> Result<()> {
        match event {
            P2PEvent::PeerConnected { peer_id } => {
                self.handle_peer_connected(peer_id).await?;
            }
            P2PEvent::PeerDisconnected { peer_id } => {
                self.handle_peer_disconnected(peer_id).await?;
            }
            P2PEvent::RelayReserved { relay_peer } => {
                info!("Relay reserved with {}", relay_peer);
                self.update_connection_stats().await?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle discovery events
    async fn handle_discovery_event(&mut self, event: DiscoveryEvent) -> Result<()> {
        match event {
            DiscoveryEvent::GameDiscovered(game) => {
                self.ui_event_tx.send(P2PIntegrationEvent::GameDiscovered {
                    game_id: game.id,
                    host_name: game.metadata.host_name,
                    board_size: game.metadata.board_size,
                    requires_relay: game.connection_quality.relay_required,
                })?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle peer connection
    async fn handle_peer_connected(&mut self, peer_id: PeerId) -> Result<()> {
        debug!("Peer connected: {}", peer_id);
        
        // Check if this peer is part of any game session
        let mut sessions = self.game_sessions.write().await;
        for (game_id, session) in sessions.iter_mut() {
            if session.remote_peer == peer_id && session.connection_state != ConnectionState::Connected {
                info!("Game {} peer reconnected", game_id);
                session.connection_state = ConnectionState::Connected;
                session.reconnect_attempts = 0;
                
                self.ui_event_tx.send(P2PIntegrationEvent::GameConnected {
                    game_id: game_id.clone(),
                    peer_id,
                })?;
            }
        }
        
        self.update_connection_stats().await?;
        Ok(())
    }
    
    /// Handle peer disconnection
    async fn handle_peer_disconnected(&mut self, peer_id: PeerId) -> Result<()> {
        warn!("Peer disconnected: {}", peer_id);
        
        // Mark affected game sessions as disconnected
        let mut sessions = self.game_sessions.write().await;
        for (game_id, session) in sessions.iter_mut() {
            if session.remote_peer == peer_id && session.connection_state == ConnectionState::Connected {
                info!("Game {} peer disconnected, will attempt reconnection", game_id);
                session.connection_state = ConnectionState::Disconnected;
                
                self.ui_event_tx.send(P2PIntegrationEvent::GameDisconnected {
                    game_id: game_id.clone(),
                    reason: "Peer disconnected".to_string(),
                })?;
            }
        }
        
        self.update_connection_stats().await?;
        Ok(())
    }
    
    /// Create a new game
    pub async fn create_game(&self, game_id: GameId, channel: Arc<GameChannel>, metadata: GameMetadata) -> Result<()> {
        info!("Creating game {} on P2P network", game_id);
        
        // Publish game to network
        let mut node = self.node.write().await;
        node.publish_game(&game_id, metadata).await?;
        
        Ok(())
    }
    
    /// Join a game
    pub async fn join_game(
        &self,
        game_id: GameId,
        host_peer: PeerId,
        channel: Arc<GameChannel>,
    ) -> Result<()> {
        info!("Joining game {} hosted by {}", game_id, host_peer);
        
        // Create game session
        let session = GameSession {
            game_id: game_id.clone(),
            channel,
            remote_peer: host_peer,
            connection_state: ConnectionState::Disconnected,
            reconnect_attempts: 0,
        };
        
        self.game_sessions.write().await.insert(game_id.clone(), session);
        
        // Connect to host
        self.connect_to_game_peer(host_peer).await?;
        
        Ok(())
    }
    
    /// Connect to a game peer
    async fn connect_to_game_peer(&self, peer_id: PeerId) -> Result<()> {
        let mut node = self.node.write().await;
        
        // First, check if we need a relay
        let connected_peers = node.connected_peers().await;
        if connected_peers.contains(&peer_id) {
            info!("Already connected to {}", peer_id);
            return Ok(());
        }
        
        // Try to connect
        // The node will automatically use relay if needed
        node.connect_peer(peer_id, vec![]).await?;
        
        Ok(())
    }
    
    /// Leave a game
    pub async fn leave_game(&self, game_id: &GameId) -> Result<()> {
        info!("Leaving game {}", game_id);
        
        self.game_sessions.write().await.remove(game_id);
        self.update_connection_stats().await?;
        
        Ok(())
    }
    
    /// Update connection statistics
    async fn update_connection_stats(&self) -> Result<()> {
        let node = self.node.read().await;
        let sessions = self.game_sessions.read().await;
        
        let connected_peers = node.connected_peers().await.len();
        let active_games = sessions.values()
            .filter(|s| s.connection_state == ConnectionState::Connected)
            .count();
        let relay_active = !node.known_relays().await.is_empty();
        
        self.ui_event_tx.send(P2PIntegrationEvent::ConnectionStats {
            connected_peers,
            active_games,
            relay_active,
        })?;
        
        Ok(())
    }
    
    /// Get relay mode
    pub fn relay_mode(&self) -> RelayMode {
        // This would be retrieved from config
        RelayMode::Normal
    }
    
    /// Set relay mode
    pub async fn set_relay_mode(&mut self, mode: RelayMode) -> Result<()> {
        info!("Setting relay mode to {:?}", mode);
        
        // In a full implementation, this would reconfigure the P2P node
        // For now, just log the change
        
        Ok(())
    }
}

/// Reconnection manager task
async fn reconnection_manager(
    sessions: Arc<RwLock<HashMap<GameId, GameSession>>>,
    node: Arc<RwLock<P2PNode>>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
    
    loop {
        interval.tick().await;
        
        // Check for disconnected sessions
        let mut to_reconnect = vec![];
        {
            let sessions = sessions.read().await;
            for (game_id, session) in sessions.iter() {
                if session.connection_state == ConnectionState::Disconnected {
                    to_reconnect.push((game_id.clone(), session.remote_peer));
                }
            }
        }
        
        // Attempt reconnections
        for (game_id, peer_id) in to_reconnect {
            debug!("Attempting to reconnect game {} to peer {}", game_id, peer_id);
            
            // Update state
            {
                let mut sessions = sessions.write().await;
                if let Some(session) = sessions.get_mut(&game_id) {
                    session.connection_state = ConnectionState::Reconnecting;
                    session.reconnect_attempts += 1;
                    
                    // Give up after 10 attempts
                    if session.reconnect_attempts > 10 {
                        warn!("Failed to reconnect game {} after 10 attempts", game_id);
                        session.connection_state = ConnectionState::Failed;
                        continue;
                    }
                }
            }
            
            // Try to reconnect
            let mut node = node.write().await;
            if let Err(e) = node.connect_peer(peer_id, vec![]).await {
                warn!("Reconnection attempt failed for game {}: {}", game_id, e);
            }
        }
    }
}