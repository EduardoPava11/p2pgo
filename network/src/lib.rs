// SPDX-License-Identifier: MIT OR Apache-2.0

// #![deny(warnings)] // TODO: Re-enable after fixing all warnings
// #![deny(clippy::all)] // TODO: Re-enable after fixing all warnings

//! P2P Go Network - libp2p-based networking layer for MVP
//!
//! This crate provides the networking functionality including:
//! - Direct peer connections with Circuit Relay v2
//! - RNA-based discovery and training data sharing
//! - Game lobby for discovering and joining games
//! - Bootstrap status and connection monitoring
//! - Auto-update mechanism

#![deny(unsafe_code)]

use libp2p::{Multiaddr, PeerId};
use p2pgo_core;

pub mod auto_update;
pub mod behaviour;
pub mod benchmark;
pub mod bootstrap;
pub mod bootstrap_relay;
pub mod circuit_relay_v2;
pub mod net_util;
pub mod relay_mesh;
pub mod relay_node;
pub mod relay_provider;
pub mod relay_server;
pub mod rna;
pub mod simple_relay;

// Keep existing modules that don't depend on iroh
pub mod blob_store;
pub mod config;
pub mod connection_manager;
pub mod game_channel;
pub mod game_classifier;
pub mod guilds;
pub mod health;
pub mod lobby;
pub mod message_security;
pub mod neural_marketplace;
pub mod port;
pub mod protocols;
pub mod relay_monitor;
pub mod relay_robustness;
pub mod smart_load_balancer;
// pub mod p2p_behaviour;  // TODO: Fix Option<Behaviour> issue
pub mod p2p_behaviour_simple;
// pub mod p2p_node; // TODO: Fix libp2p NetworkBehaviour compilation
// pub mod game_discovery; // TODO: Fix after p2p_node
// pub mod p2p_integration; // TODO: Fix after p2p_node
pub mod relay_config;
pub mod simple_p2p;

// Type aliases
pub type GameId = String;
pub type NodeContext = P2PContext;

// BlobHash type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobHash([u8; 32]);

impl BlobHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

// P2P Context for libp2p-based networking
#[derive(Clone)]
pub struct P2PContext {
    peer_id: libp2p::PeerId,
    addresses: Vec<libp2p::Multiaddr>,
}

impl P2PContext {
    pub fn new(peer_id: libp2p::PeerId) -> Self {
        Self {
            peer_id,
            addresses: Vec::new(),
        }
    }

    pub fn peer_id(&self) -> libp2p::PeerId {
        self.peer_id
    }

    pub fn node_id(&self) -> String {
        self.peer_id.to_string()
    }

    pub fn add_address(&mut self, addr: libp2p::Multiaddr) {
        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }
    }

    pub fn addresses(&self) -> &[libp2p::Multiaddr] {
        &self.addresses
    }

    pub fn ticket(&self) -> String {
        // Create a multiaddr-based ticket for connection
        if let Some(addr) = self.addresses.first() {
            format!("{}/p2p/{}", addr, self.peer_id)
        } else {
            self.peer_id.to_string()
        }
    }

    // Stub methods for compatibility
    pub async fn advertise_game(&self, _game_info: &crate::lobby::GameInfo) -> anyhow::Result<()> {
        // TODO: Implement game advertisement via libp2p gossipsub
        Ok(())
    }

    pub async fn store_move_tag(&self, _game_id: &str, _tag: &str) -> anyhow::Result<()> {
        // TODO: Implement move tag storage
        Ok(())
    }

    pub async fn connect_by_ticket(&self, _ticket: &str) -> anyhow::Result<()> {
        // TODO: Implement ticket-based connection via libp2p multiaddr
        Ok(())
    }

    pub async fn store_game_move(
        &self,
        _game_id: &str,
        _mv: p2pgo_core::Move,
    ) -> anyhow::Result<()> {
        // TODO: Implement move storage
        Ok(())
    }
}

// Re-exports
pub use auto_update::{AutoUpdater, UpdateConfig};
pub use behaviour::{Event, P2PGoBehaviour};
pub use bootstrap::{Bootstrap, BootstrapConfig};
pub use bootstrap_relay::{run_bootstrap_relay, BootstrapRelay};
pub use relay_node::RelayNode;
pub use rna::{RNAMessage, RNAType};
