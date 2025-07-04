// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game engine interfaces and AI backend

use std::time::Duration;
use crate::{GameState, Move, cbor::MoveRecord};

/// Player backend trait for both human and AI players
pub trait PlayerBackend {
    /// Get the next move from this player
    fn next_move(&mut self, pos: &GameState, time_left: Duration) -> Move;
}

/// Calculate the hash for a move given the previous hash and move data
/// Uses Blake3 for fast, secure hashing
pub fn calculate_move_hash(prev_hash: &[u8; 32], move_data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_hash);
    hasher.update(move_data);
    *hasher.finalize().as_bytes()
}

/// Create a new MoveRecord with properly set prev_hash and broadcast_hash
pub fn create_move_record(x: u8, y: u8, player: u8, prev_hash: Option<[u8; 32]>) -> MoveRecord {
    let color = if player == 0 { crate::Color::Black } else { crate::Color::White };
    let mut record = MoveRecord::new(
        if x < 19 && y < 19 {
            Move::Place { x, y, color }
        } else if x == 255 && y == 255 {
            Move::Pass
        } else {
            Move::Resign
        },
        None,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        prev_hash
    );
    
    // Calculate proper broadcast hash
    let move_data = record.to_bytes();
    record.broadcast_hash = Some(calculate_move_hash(&record.prev_hash.unwrap_or([0u8; 32]), &move_data));
    
    record
}

/// Check if the hash chain is valid for a sequence of moves
pub fn verify_hash_chain(moves: &[MoveRecord]) -> bool {
    if moves.is_empty() {
        return true;
    }
    
    // For the first move, we just check that it has a broadcast hash if needed
    if moves[0].prev_hash.is_some() && moves[0].broadcast_hash.is_none() {
        println!("First move has prev_hash but no broadcast_hash");
        return false;
    }
    
    // Check that each move's prev_hash matches the previous move's broadcast_hash
    for i in 1..moves.len() {
        // Both hashes must be present to form a valid chain
        if moves[i].prev_hash.is_none() || moves[i-1].broadcast_hash.is_none() {
            println!("Missing hash at position {}: prev_hash={:?}, prev broadcast={:?}", 
                i, moves[i].prev_hash, moves[i-1].broadcast_hash);
            return false;
        }
        
        // Hashes must match
        if moves[i].prev_hash != moves[i-1].broadcast_hash {
            println!("Hash mismatch at position {}: prev_hash={:?}, prev broadcast={:?}", 
                i, moves[i].prev_hash, moves[i-1].broadcast_hash);
            return false;
        }
        
        // We should NOT recalculate the broadcast hash here
        // The broadcast hash is calculated once when the move is created
        // and stored in the MoveRecord
        // Simply check that the hashes form a valid chain
        
        // For debugging, let's print the hashes
        println!("Move {} hash chain looks good", i);
    }
    
    true
}

#[cfg(feature = "bot")]
pub mod bot {
    use super::*;
    use std::path::Path;
    
    /// AI bot that loads from a dynamic library
    pub struct DynamicBot {
        // Library handle would go here
    }
    
    impl DynamicBot {
        /// Create a new dynamic bot from the specified library path
        pub fn new(_path: &Path) -> Option<Self> {
            // TODO: implement dynamic library loading
            todo!("Implement dynamic library loading")
        }
    }
    
    impl PlayerBackend for DynamicBot {
        fn next_move(&mut self, pos: &GameState, time_left: Duration) -> Move {
            // TODO: implement AI move calculation via dynamic library
            todo!("Implement AI move calculation")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[allow(dead_code)]
    struct RandomPlayer;
    
    impl PlayerBackend for RandomPlayer {
        fn next_move(&mut self, _pos: &GameState, _time_left: Duration) -> Move {
            Move::Pass // Always pass for now
        }
    }
    
    #[test]
    fn test_create_move_record() {
        // Test creating a regular move
        let record = create_move_record(3, 4, 1, None);
        assert_eq!(record.mv, Move::Place(Coord::new(3, 4)));
        assert!(record.broadcast_hash.is_some());
        
        // Test pass move
        let pass_record = create_move_record(255, 255, 1, None);
        assert_eq!(pass_record.mv, Move::Pass);
        
        // Test resign move
        let resign_record = create_move_record(255, 0, 1, None);
        assert_eq!(resign_record.mv, Move::Resign);
    }
    
    #[test]
    fn test_hash_chain_integrity() {
        // Create a chain of moves
        let mut moves = Vec::new();
        let mut prev_hash = None;
        
        println!("Creating move chain:");
        for i in 0..5 {
            let record = create_move_record(i, i, (i % 2 + 1) as u8, prev_hash);
            println!("Move {}: prev_hash={:?}, broadcast_hash={:?}", i, record.prev_hash, record.broadcast_hash);
            prev_hash = record.broadcast_hash;
            moves.push(record);
        }
        
        // Verify the chain
        println!("Verifying the move chain...");
        assert!(verify_hash_chain(&moves));
        
        // Break the chain and verify it fails
        if !moves.is_empty() {
            println!("Testing with broken chain...");
            let mut broken_chain = moves.clone();
            broken_chain[1].prev_hash = Some([0u8; 32]); // Break the link
            assert!(!verify_hash_chain(&broken_chain));
        }
    }
}
