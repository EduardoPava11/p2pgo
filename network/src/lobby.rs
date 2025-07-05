// SPDX-License-Identifier: MIT OR Apache-2.0

//! In-process lobby implementation for MVP.
//!   * create_game / start_game / get_game_channel
//!   * broadcast LobbyEvent via tokio::sync::broadcast

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use anyhow::Result;
use p2pgo_core::{GameState, Move};
use crate::GameId;
use crate::game_channel::GameChannel;
use serde::{Serialize, Deserialize};

/// Bot information for lobby advertisements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotInfo {
    pub layers: u8,
    pub d_model: u16,
    pub quant: u8,
    pub sha8: [u8; 8],
}

/// Game advertisement for iroh-gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAdvert {
    pub gid: GameId,
    pub size: u8,
    pub host: String, // NodeId as string
    pub bot: Option<BotInfo>,
}

/// Information about a game in the lobby
#[derive(Debug, Clone)]
pub struct GameInfo {
    /// Unique identifier for the game
    pub id: GameId,
    /// Name of the game (if any)
    pub name: Option<String>,
    /// Board size
    pub board_size: u8,
    /// Whether the game has started
    pub started: bool,
    /// Whether the game needs a password to join
    pub needs_password: bool,
}

/// Events emitted by the lobby
#[derive(Debug, Clone)]
pub enum LobbyEvent {
    /// A new game was created
    GameCreated(GameInfo),
    /// A game was started
    GameStarted(GameId),
    /// A game was ended
    GameEnded(GameId),
    /// A player joined a game
    PlayerJoined {
        /// Game ID
        game_id: GameId,
        /// Player color
        color: p2pgo_core::Color,
    },
}

/// Service for managing the game lobby
pub struct Lobby {
    /// Currently available games
    games: Arc<RwLock<HashMap<GameId, GameInfo>>>,
    /// Game channels
    channels: Arc<RwLock<HashMap<GameId, Arc<GameChannel>>>>,
    /// Lobby event broadcaster
    events_tx: broadcast::Sender<LobbyEvent>,
    /// Keep a receiver alive to prevent channel closure
    _events_rx: broadcast::Receiver<LobbyEvent>,
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}

impl Lobby {
    /// Create a new lobby service
    pub fn new() -> Self {
        let _span = tracing::info_span!("network.lobby", "Lobby::new").entered();
        
        // Create a broadcast channel for events with buffer size 100
        // We must ensure there's always at least one active receiver for tests to pass
        let (events_tx, events_rx) = broadcast::channel(100);
        
        Self {
            games: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            events_tx,
            _events_rx: events_rx,
        }
    }
    
    /// Get a receiver for lobby events
    pub fn subscribe(&self) -> broadcast::Receiver<LobbyEvent> {
        self.events_tx.subscribe()
    }
    
    /// Create a new game in the lobby
    pub async fn create_game(&self, name: Option<String>, board_size: u8, needs_password: bool) -> Result<GameId> {
        let _span = tracing::info_span!("network.lobby", "Lobby::create_game").entered();
        
        // Generate a unique game ID
        let game_id = format!("game-{}", uuid::Uuid::new_v4());
        
        // Create initial game state with default board size 9 if None
        let board_size = if board_size == 0 { 9 } else { board_size };
        let initial_state = GameState::new(board_size);
        
        // Create game info
        let game_info = GameInfo {
            id: game_id.clone(),
            name,
            board_size,
            started: false,
            needs_password,
        };
        
        // Create a game channel
        let channel = Arc::new(GameChannel::new(game_id.clone(), initial_state));
        
        // Add to local games map and channels
        {
            let mut games = self.games.write().await;
            games.insert(game_id.clone(), game_info.clone());
            
            let mut channels = self.channels.write().await;
            channels.insert(game_id.clone(), channel);
        }
        
        // Broadcast the game created event
        let event = LobbyEvent::GameCreated(game_info);
        tracing::debug!(
            game_id = %game_id,
            event_type = "GameCreated",
            board_size = board_size,
            needs_password = needs_password,
            "Broadcasting lobby event"
        );
        self.events_tx.send(event)
            .map_err(|e| anyhow::anyhow!("Failed to broadcast game created event: {}", e))?;
        
        Ok(game_id)
    }
    
    /// Start a game
    pub async fn start_game(&self, game_id: &GameId) -> Result<()> {
        let _span = tracing::info_span!("network.lobby", "Lobby::start_game").entered();
        
        // Update the game info
        {
            let mut games = self.games.write().await;
            if let Some(game_info) = games.get_mut(game_id) {
                game_info.started = true;
            } else {
                return Err(anyhow::anyhow!("Game not found: {}", game_id));
            }
        }
        
        // Broadcast the game started event
        let event = LobbyEvent::GameStarted(game_id.clone());
        tracing::debug!(
            game_id = %game_id,
            event_type = "GameStarted",
            "Broadcasting lobby event"
        );
        self.events_tx.send(event)
            .map_err(|e| anyhow::anyhow!("Failed to broadcast game started event: {}", e))?;
        
        Ok(())
    }
    
    /// Get a game channel for a specific game
    pub async fn get_game_channel(&self, game_id: &GameId) -> Result<Arc<GameChannel>> {
        let channels = self.channels.read().await;
        channels.get(game_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Game not found: {}", game_id))
    }
    
    /// Post a move to a game
    pub async fn post_move(&self, game_id: &GameId, mv: Move) -> Result<()> {
        let channel = self.get_game_channel(game_id).await?;
        channel.send_move(mv).await
    }
    
    /// List all available games
    pub async fn list_games(&self) -> Vec<GameInfo> {
        let games = self.games.read().await;
        games.values().cloned().collect()
    }
    
    /// Remove a game from the lobby
    pub async fn remove_game(&self, game_id: &GameId) -> Result<()> {
        let _span = tracing::info_span!("network.lobby", "Lobby::remove_game").entered();
        
        // Remove from local maps
        {
            let mut games = self.games.write().await;
            games.remove(game_id);
            
            let mut channels = self.channels.write().await;
            channels.remove(game_id);
        }
        
        // Broadcast the game ended event
        let event = LobbyEvent::GameEnded(game_id.clone());
        tracing::debug!(
            game_id = %game_id,
            event_type = "GameEnded",
            "Broadcasting lobby event"
        );
        self.events_tx.send(event)
            .map_err(|e| anyhow::anyhow!("Failed to broadcast game ended event: {}", e))?;
        
        Ok(())
    }
    
    /// Publish game advertisement via gossip
    pub async fn publish_game_advert(&self, game_id: &GameId, host_node_id: &str, bot_info: Option<BotInfo>) -> Result<()> {
        let _span = tracing::info_span!("network.lobby", "Lobby::publish_game_advert").entered();
        
        // Get game info
        let game_info = {
            let games = self.games.read().await;
            games.get(game_id).cloned()
        };
        
        if let Some(info) = game_info {
            let has_bot = bot_info.is_some();
            let advert = GameAdvert {
                gid: game_id.clone(),
                size: info.board_size,
                host: host_node_id.to_string(),
                bot: bot_info,
            };
            
            // Serialize using bincode for gossip
            let data = bincode::serialize(&advert)
                .map_err(|e| anyhow::anyhow!("Failed to serialize game advert: {}", e))?;
            
            tracing::debug!(
                game_id = %game_id,
                host = %host_node_id,
                board_size = info.board_size,
                has_bot = has_bot,
                data_len = data.len(),
                "Publishing game advertisement"
            );
            
            // TODO: Actually publish to iroh-gossip when available
            // For now just log the advertisement
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Game not found: {}", game_id))
        }
    }
    
    /// Create bot info for local AI player
    pub fn create_bot_info(layers: u8, d_model: u16, quant: u8, model_hash: &[u8]) -> BotInfo {
        let mut sha8 = [0u8; 8];
        let len = std::cmp::min(model_hash.len(), 8);
        sha8[..len].copy_from_slice(&model_hash[..len]);
        
        BotInfo {
            layers,
            d_model,
            quant,
            sha8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::{Coord, Color, GameEvent};
    
    #[tokio::test]
    async fn test_lobby_create_game() {
        let lobby = Lobby::new();
        
        // Subscribe to events first
        let _rx = lobby.subscribe();
        
        // Create a game
        let game_id = lobby.create_game(Some("Test Game".to_string()), 9, false)
            .await
            .unwrap();
        
        // Check the game is in the list
        let games = lobby.list_games().await;
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, game_id);
        assert_eq!(games[0].name, Some("Test Game".to_string()));
        assert_eq!(games[0].board_size, 9);
        assert!(!games[0].started);
    }
    
    #[tokio::test]
    async fn test_lobby_start_game() {
        let lobby = Lobby::new();
        
        // Subscribe to events first
        let mut rx = lobby.subscribe();
        
        // Create a game
        let game_id = lobby.create_game(Some("Test Game".to_string()), 9, false)
            .await
            .unwrap();
        
        // Check game created event
        let event = rx.recv().await.unwrap();
        match event {
            LobbyEvent::GameCreated(info) => {
                assert_eq!(info.id, game_id);
            },
            _ => panic!("Expected GameCreated event"),
        }
            
        // Start the game
        lobby.start_game(&game_id).await.unwrap();
        
        // Check game started event
        let event = rx.recv().await.unwrap();
        match event {
            LobbyEvent::GameStarted(id) => {
                assert_eq!(id, game_id);
            },
            _ => panic!("Expected GameStarted event"),
        }
        
        // Check the game is marked as started
        let games = lobby.list_games().await;
        assert!(games[0].started);
    }
    
    #[tokio::test]
    async fn test_lobby_post_move() {
        let lobby = Lobby::new();
        
        // Subscribe to lobby events first
        let mut lobby_rx = lobby.subscribe();
        
        // Create a game
        let game_id = lobby.create_game(None, 9, false)
            .await
            .unwrap();
            
        // Check for game created event
        let event = lobby_rx.recv().await.unwrap();
        match event {
            LobbyEvent::GameCreated(info) => {
                assert_eq!(info.id, game_id);
            },
            _ => panic!("Expected GameCreated event"),
        }
        
        // Start the game
        lobby.start_game(&game_id).await.unwrap();
        
        // Check for game started event
        let event = lobby_rx.recv().await.unwrap();
        match event {
            LobbyEvent::GameStarted(id) => {
                assert_eq!(id, game_id);
            },
            _ => panic!("Expected GameStarted event"),
        }
        
        // Get a channel to listen for game events
        let channel = lobby.get_game_channel(&game_id).await.unwrap();
        let mut game_rx = channel.subscribe();
        
        // Post a move
        let mv = Move::Place { x: 4, y: 4, color: Color::Black };
        lobby.post_move(&game_id, mv.clone()).await.unwrap();
        
        // Check that the move event was received
        let event = game_rx.recv().await.unwrap();
        match event {
            GameEvent::MoveMade { mv: received_mv, by } => {
                assert_eq!(received_mv, mv);
                assert_eq!(by, Color::Black); // First move is by Black
            },
            _ => panic!("Expected MoveMade event"),
        }
    }
}

#[cfg(feature = "iroh")]
mod iroh_integration {
    //! Thin wrappers around iroh-net once we add real P2P tests.
}