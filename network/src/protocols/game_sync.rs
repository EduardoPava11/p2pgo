//! Game state synchronization protocol for P2P Go
//!
//! Implements a decentralized game state synchronization protocol
//! that allows peers to maintain consistent game state without
//! relying on a central server.

use libp2p::PeerId;
use p2pgo_core::{GameState, Move};
use serde::{Deserialize, Serialize};

/// Protocol name for game synchronization
pub const GAME_SYNC_PROTOCOL: &str = "/p2pgo/sync/1.0.0";

/// Game synchronization protocol handler
pub struct GameSyncProtocol;

/// Game sync request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameSyncRequest {
    /// Request full game state
    GetGameState { game_id: String },

    /// Submit a new move (gossip-style)
    ProposeMove {
        game_id: String,
        move_data: Move,
        signature: Vec<u8>,
    },

    /// Request game history
    GetGameHistory { game_id: String, from_move: u32 },

    /// Subscribe to game updates
    SubscribeGame { game_id: String },
}

/// Game sync response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameSyncResponse {
    /// Full game state
    GameState {
        game_id: String,
        state: GameState,
        move_history: Vec<Move>,
        participants: Vec<String>,
    },

    /// Move accepted/rejected
    MoveResult {
        accepted: bool,
        reason: Option<String>,
        new_state_hash: [u8; 32],
    },

    /// Game history
    GameHistory {
        moves: Vec<(u32, Move, Vec<u8>)>, // (move_num, move, signature)
    },

    /// Subscription confirmed
    Subscribed { current_move: u32 },

    /// Error response
    Error { message: String },
}

/// Codec for game sync protocol
#[derive(Debug, Clone, Default)]
pub struct GameSyncCodec;

impl GameSyncProtocol {
    /// Create a new game sync protocol handler
    pub fn new() -> Self {
        Self
    }

    /// Validate move signatures to ensure game integrity
    pub fn validate_move_signature(
        &self,
        _game_id: &str,
        _move_data: &Move,
        signature: &[u8],
        _peer_id: &PeerId,
    ) -> bool {
        // In a real implementation, this would verify cryptographic signatures
        // For now, we'll implement a simple check
        // In a real implementation, verify ed25519 signature
        // For now just check basic validity
        !signature.is_empty()
    }

    /// Calculate state hash for consensus
    pub fn calculate_state_hash(state: &GameState) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // Hash the board state
        for row in 0..state.board_size {
            for col in 0..state.board_size {
                let idx = (row * state.board_size + col) as usize;
                match state.board.get(idx) {
                    Some(Some(color)) => {
                        hasher.update(&[row, col, color.clone() as u8]);
                    }
                    _ => {
                        hasher.update(&[row, col, 255]);
                    }
                }
            }
        }

        // Hash game metadata
        hasher.update(&state.moves.len().to_be_bytes());
        hasher.update(&[state.current_player.clone() as u8]);

        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[..32]);
        result
    }
}
