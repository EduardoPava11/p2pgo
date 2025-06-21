// SPDX-License-Identifier: MIT OR Apache-2.0

//! P2P Go Network - Iroh-based networking layer
//!
//! This crate provides the networking functionality including:
//! - Game lobby for discovering and joining games
//! - Game channels for player communication
//! - Blob storage for game state synchronization

#![deny(unsafe_code)]

pub mod lobby;
pub mod game_channel;
pub mod blob_store;
pub mod iroh_endpoint;
pub mod archive;
pub mod gossip_compat;
pub mod crash_logger;
pub mod config;
pub mod relay_monitor;

// Re-export key types for convenience
pub use lobby::Lobby;
pub use game_channel::GameChannel;
pub use iroh_endpoint::IrohCtx;
pub use archive::ArchiveManager;
pub use crash_logger::{init_crash_logger, log_crash, get_crash_logger_stats};

use std::fmt;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Unique identifier for a game session
pub type GameId = String;

/// A hash for content-addressed blob storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobHash([u8; 32]);

impl BlobHash {
    /// Create a new blob hash from raw bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Get the raw bytes of the hash
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for BlobHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0[..6] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "...")
    }
}

/// Dummy Iroh implementation for now
pub struct DummyIroh;

impl Default for DummyIroh {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyIroh {
    /// Create a new dummy Iroh instance
    pub fn new() -> Self {
        Self
    }
    
    /// Mock method for connecting to the Iroh network
    pub async fn connect(&self) -> Result<(), NetworkError> {
        // Just a placeholder
        Ok(())
    }
}

/// Errors that can occur in the network layer
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Failed to connect to a peer
    #[error("Failed to connect to peer: {0}")]
    ConnectionFailed(String),
    
    /// Failed to send data
    #[error("Failed to send data: {0}")]
    SendFailed(String),
    
    /// Failed to receive data
    #[error("Failed to receive data: {0}")]
    ReceiveFailed(String),
    
    /// Data serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Game not found
    #[error("Game not found: {0}")]
    GameNotFound(uuid::Uuid),
}

#[cfg(any(test, feature = "headless"))]
pub mod debug {
    use p2pgo_core::GameState;
    use std::sync::{Arc, Mutex};
    use std::sync::OnceLock;
    
    static LATEST_STATE: OnceLock<Arc<Mutex<Option<GameState>>>> = OnceLock::new();
    
    fn get_state_store() -> &'static Arc<Mutex<Option<GameState>>> {
        LATEST_STATE.get_or_init(|| Arc::new(Mutex::new(None)))
    }
    
    pub fn store_latest_reconstructed(state: GameState) {
        if let Ok(mut latest) = get_state_store().lock() {
            *latest = Some(state);
        }
    }
    
    pub fn latest_reconstructed() -> GameState {
        if let Ok(latest) = get_state_store().lock() {
            latest.clone().unwrap_or_else(|| GameState::new(9))
        } else {
            GameState::new(9)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore = "Implementation needed"]
    async fn test_dummy_iroh() {
        let iroh = DummyIroh::new();
        assert!(iroh.connect().await.is_ok());
    }
}
