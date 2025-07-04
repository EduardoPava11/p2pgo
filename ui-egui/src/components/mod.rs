//! Modular UI components organized by feature

pub mod board;
pub mod game;
pub mod network;

// Re-export commonly used components
pub use board::{BoardRenderer, BoardInteraction};
pub use game::{GameControls, GameStatus};
pub use network::{ConnectionStatus, PeerList};