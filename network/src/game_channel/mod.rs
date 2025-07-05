// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game channel for communication between players
//!
//! This module provides a modular game channel system that handles:
//! - Game state management and move validation
//! - P2P communication and synchronization
//! - Event broadcasting and subscription
//! - Cryptographic verification of moves
//! - Snapshot persistence and recovery
//! - Connection management and reliability

use crate::blob_store::MoveChain;
use crate::{GameId, NodeContext};
use anyhow::Result;
use libp2p::PeerId;
use p2pgo_core::{GameEvent, GameState, Move};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

// Module declarations
pub mod core;
pub mod crypto;
pub mod messages;
pub mod networking;
pub mod registry;
pub mod state;
pub mod storage;
pub mod sync;

// Re-exports
pub use messages::*;
pub use registry::GameChannelRegistry;

/// Status of a player in the game
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerStatus {
    /// Player has connected but not yet joined
    Connected,
    /// Player has joined the game as a specific color
    Playing(p2pgo_core::Color),
    /// Player is watching the game (not playing)
    Watching,
}

/// A channel for game-related communication
pub struct GameChannel {
    /// Game ID
    pub(crate) game_id: GameId,
    /// Move chain for storing game history
    pub(crate) move_chain: Arc<RwLock<MoveChain>>,
    /// Event broadcast channel
    pub(crate) events_tx: broadcast::Sender<GameEvent>,
    /// Latest game state
    pub(crate) latest_state: Arc<RwLock<Option<GameState>>>,

    /// Timestamp when the last snapshot was written
    pub(crate) last_snapshot_time: Arc<RwLock<std::time::Instant>>,

    /// Number of moves since the last snapshot was written
    pub(crate) moves_since_snapshot: Arc<RwLock<u32>>,

    /// Directory to store game snapshots
    pub(crate) snapshot_dir: Arc<RwLock<Option<std::path::PathBuf>>>,

    /// P2P networking context
    pub(crate) node_ctx: Option<Arc<NodeContext>>,

    /// Active peer connections for this game
    pub(crate) peer_connections: Arc<RwLock<HashMap<PeerId, bool>>>,

    /// Background task for handling incoming messages
    pub(crate) _message_handler_task: Option<JoinHandle<()>>,

    /// Queue of already processed (PeerId, sequence) pairs to avoid duplicates
    pub(crate) processed_sequences: Arc<Mutex<VecDeque<(PeerId, u64)>>>,

    /// Index of the last move sent
    pub(crate) last_sent_index: Arc<RwLock<Option<usize>>>,

    /// Timestamp when the last move was sent
    pub(crate) last_sent_time: Arc<RwLock<Option<std::time::Instant>>>,

    /// Flag to track if sync has been requested for the current move
    pub(crate) sync_requested: Arc<RwLock<bool>>,
}

// Core API implementation
impl GameChannel {
    /// Create a new game channel
    pub fn new(game_id: GameId, initial_state: GameState) -> Self {
        core::new(game_id, initial_state)
    }

    /// Create a new game channel with an Iroh context for network synchronization
    #[cfg(feature = "iroh")]
    pub async fn with_iroh(
        game_id: GameId,
        initial_state: GameState,
        iroh_ctx: Arc<IrohCtx>,
    ) -> Result<Self> {
        core::with_iroh(game_id, initial_state, iroh_ctx).await
    }

    /// Get a receiver for game events
    pub fn subscribe(&self) -> broadcast::Receiver<GameEvent> {
        self.events_tx.subscribe()
    }

    /// Send a move to the channel
    pub async fn send_move(&self, mv: Move) -> Result<()> {
        state::send_move(self, mv).await
    }

    /// Handle a local move - this is a public method used by the CLI and tests
    pub async fn handle_local_move(&self, mv: Move) -> Result<()> {
        state::handle_local_move(self, mv).await
    }

    /// Get the game ID for this channel
    pub fn get_game_id(&self) -> &str {
        &self.game_id
    }

    /// Get the latest game state
    pub async fn get_latest_state(&self) -> Option<GameState> {
        state::get_latest_state(self).await
    }

    /// Get all moves in the game so far
    pub async fn get_all_moves(&self) -> Vec<Move> {
        state::get_all_moves(self).await
    }

    /// Send a chat message or other game event
    pub async fn send_event(&self, event: GameEvent) -> Result<()> {
        state::send_event(self, event).await
    }

    /// Set the directory where game snapshots will be saved
    pub async fn set_snapshot_directory(&self, dir_path: std::path::PathBuf) -> Result<()> {
        storage::set_snapshot_directory(self, dir_path).await
    }

    /// Connect to a peer's game channel for the same game
    #[cfg(feature = "iroh")]
    pub async fn connect_to_peer(&self, peer_ticket: &str) -> Result<()> {
        networking::connect_to_peer(self, peer_ticket).await
    }

    /// Broadcast move to peers over direct connections
    #[cfg(feature = "iroh")]
    pub async fn broadcast_move_to_peers(
        &self,
        move_record: &p2pgo_core::MoveRecord,
    ) -> Result<()> {
        networking::broadcast_move_to_peers(self, move_record).await
    }

    /// Check if we need to request a sync due to missing ACKs
    #[cfg(feature = "iroh")]
    pub async fn check_sync_timeouts(&self) -> Result<()> {
        sync::check_sync_timeouts(self).await
    }

    /// Request a sync of game state from peers
    #[cfg(feature = "iroh")]
    pub async fn request_sync(&self) -> Result<()> {
        sync::request_sync(self).await
    }
}

// Test helpers
#[cfg(feature = "iroh")]
impl GameChannel {
    /// Test helper method to get all move records including signatures
    #[cfg(test)]
    pub async fn get_all_move_records(&self) -> Vec<p2pgo_core::MoveRecord> {
        state::get_all_move_records(self).await
    }

    /// Test helper method to handle a duplicate move for testing deduplication
    #[cfg(test)]
    pub async fn handle_duplicate_move_test(
        &self,
        move_record: p2pgo_core::MoveRecord,
    ) -> Result<()> {
        state::handle_duplicate_move_test(self, move_record).await
    }

    /// Test helper to check if a move was deduplicated
    #[cfg(test)]
    pub async fn was_move_deduplicated(&self) -> bool {
        state::was_move_deduplicated(self).await
    }
}

// Registry functionality
impl GameChannel {
    /// Register a game channel
    #[cfg(feature = "iroh")]
    pub fn register(game_id: &str, channel: &std::sync::Arc<GameChannel>) {
        registry::register(game_id, channel)
    }

    /// Get a game channel by game ID
    #[cfg(feature = "iroh")]
    pub async fn get_for_game_id(game_id: &str) -> Option<std::sync::Arc<GameChannel>> {
        registry::get_for_game_id(game_id).await
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::{Coord, GameState, Move};

    #[tokio::test]
    async fn test_basic_game_channel() -> Result<()> {
        let game_id = format!("test-basic-{}", uuid::Uuid::new_v4());
        let initial_state = GameState::new(9);
        let channel = GameChannel::new(game_id.clone(), initial_state);

        // Test basic functionality
        assert_eq!(channel.get_game_id(), &game_id);
        assert!(channel.get_latest_state().await.is_some());

        // Test move sending
        let mv = Move::Place {
            x: 4,
            y: 4,
            color: p2pgo_core::Color::Black,
        };
        channel.send_move(mv.clone()).await?;

        // Verify move was processed
        let state = channel.get_latest_state().await.unwrap();
        assert_eq!(state.moves.len(), 1);

        Ok(())
    }

    #[cfg(feature = "iroh")]
    #[tokio::test]
    async fn test_ack_watchdog() -> Result<()> {
        use crate::iroh_endpoint::IrohCtx;

        // Create iroh contexts for both players
        let alice_ctx = Arc::new(IrohCtx::new().await?);
        let bob_ctx = Arc::new(IrohCtx::new().await?);

        // Create a unique game ID
        let game_id = format!("test-ack-watchdog-{}", uuid::Uuid::new_v4());
        let initial_state = GameState::new(9);

        // Create game channels
        let alice_channel =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), alice_ctx.clone())
                .await?;
        let bob_channel =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), bob_ctx.clone()).await?;

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send a move from Alice but simulate dropped ACK
        let mv = Move::Place {
            x: 4,
            y: 4,
            color: p2pgo_core::Color::Black,
        };
        alice_channel.send_move(mv.clone()).await?;

        // Wait for the watchdog to trigger (>3 seconds)
        tokio::time::sleep(Duration::from_secs(4)).await;

        // Check if sync has been requested
        let sync_requested = {
            let flag = alice_channel.sync_requested.read().await;
            *flag
        };

        assert!(
            sync_requested,
            "Sync should have been requested after ACK timeout"
        );

        Ok(())
    }
}
