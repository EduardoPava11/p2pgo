//! Core P2P protocols for decentralized game network
//! 
//! This module contains the fundamental protocols that enable
//! true peer-to-peer game coordination without central servers.

pub mod game_sync;
pub mod peer_discovery;
pub mod relay_negotiation;

pub use game_sync::GameSyncProtocol;
pub use peer_discovery::PeerDiscoveryProtocol;
pub use relay_negotiation::RelayNegotiationProtocol;