//! Modular UI components organized by feature

pub mod board;
pub mod game;
pub mod network;

// Re-export commonly used components
pub use board::{BoardInteraction, BoardRenderer};
pub use game::{GameControls, GameStatus};
pub use network::{ConnectionStatus, PeerList};
