// SPDX-License-Identifier: MIT OR Apache-2.0

#![deny(warnings)]
#![deny(clippy::all)]

//! P2P Go Network - libp2p-based networking layer for MVP
//!
//! This crate provides the networking functionality including:
//! - Direct peer connections with Circuit Relay v2
//! - RNA-based discovery and training data sharing
//! - Game lobby for discovering and joining games
//! - Bootstrap status and connection monitoring
//! - Auto-update mechanism

#![deny(unsafe_code)]

pub mod behaviour;
pub mod bootstrap;
pub mod relay_node;
pub mod auto_update;
pub mod rna;
pub mod bootstrap_relay;
pub mod benchmark;
pub mod simple_relay;

// Keep existing modules that don't depend on iroh
pub mod config;
pub mod port;
pub mod lobby;
pub mod game_channel;
pub mod neural_marketplace;
pub mod blob_store;
pub mod relay_monitor;
pub mod guilds;

// Type aliases
pub type GameId = String;
pub type IrohCtx = DummyIroh;

// BlobHash type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobHash([u8; 32]);

impl BlobHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

// Dummy type for Iroh replacement
#[derive(Clone)]
pub struct DummyIroh {
    node_id: String,
}

impl DummyIroh {
    pub fn new() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    pub fn node_id(&self) -> String {
        self.node_id.clone()
    }
    
    pub fn ticket(&self) -> String {
        format!("ticket_{}", self.node_id)
    }
    
    pub async fn connect_by_ticket(&self, _ticket: &str) -> anyhow::Result<()> {
        Ok(())
    }
    
    pub async fn store_game_move(&self, _game_id: &str, _mv: p2pgo_core::Move) -> anyhow::Result<()> {
        Ok(())
    }
    
    pub async fn advertise_game(&self, _game_info: &lobby::GameInfo) -> anyhow::Result<()> {
        Ok(())
    }
    
    pub async fn store_move_tag(&self, _game_id: &str, _tag: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

// Re-exports
pub use behaviour::{P2PGoBehaviour, Event};
pub use bootstrap::{Bootstrap, BootstrapConfig};
pub use relay_node::RelayNode;
pub use rna::{RNAMessage, RNAType};
pub use bootstrap_relay::{BootstrapRelay, run_bootstrap_relay};
pub use auto_update::{AutoUpdater, UpdateConfig};