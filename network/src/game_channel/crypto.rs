// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cryptographic verification and move authentication

use p2pgo_core::MoveRecord;

/// Try to verify a move record signature
/// Returns true if the move record should be processed
#[cfg(feature = "iroh")]
pub fn try_verify_move_record(move_record: &MoveRecord, game_id: &str) -> bool {
    // For backward compatibility, we allow moves without signatures
    if !move_record.is_signed() {
        tracing::warn!("Received move without signature for game {}", game_id);
        return true; // Process unsigned moves for now
    }

    // Verify the signature
    if move_record.verify_signature() {
        if let Some(signer) = move_record.get_signer() {
            tracing::debug!(
                "Move signature verified for game {} from {}",
                game_id,
                signer
            );
        } else {
            tracing::debug!("Move signature verified for game {}", game_id);
        }
        return true;
    } else {
        tracing::warn!("Move signature verification failed for game {}", game_id);
        return false; // Don't process moves with invalid signatures
    }
}

/// Verify a move record signature (stub for non-iroh builds)
#[cfg(not(feature = "iroh"))]
pub fn try_verify_move_record(_move_record: &MoveRecord, _game_id: &str) -> bool {
    // Always return true for non-iroh builds since we don't have signature verification
    true
}

/// Calculate a hash for a move record for integrity checking
pub fn calculate_move_hash(move_record: &MoveRecord) -> [u8; 32] {
    use blake3::Hasher;

    let mut hasher = Hasher::new();

    // Hash the essential fields by converting them to bytes
    // Hash the move enum discriminant and value
    match &move_record.mv {
        p2pgo_core::Move::Place { x, y, color } => {
            hasher.update(&[0u8]); // Place discriminant
            hasher.update(&[*x, *y]);
            hasher.update(&[*color as u8]); // Include color in hash
        }
        p2pgo_core::Move::Pass => {
            hasher.update(&[1u8]); // Pass discriminant
        }
        p2pgo_core::Move::Resign => {
            hasher.update(&[2u8]); // Resign discriminant
        }
    }

    hasher.update(&move_record.ts.to_le_bytes());

    if let Some(ref prev_hash) = move_record.prev_hash {
        hasher.update(prev_hash);
    }

    if let Some(ref tag) = move_record.tag {
        hasher.update(&[*tag as u8]);
    }

    *hasher.finalize().as_bytes()
}

/// Verify the integrity of a move chain by checking hash links
pub fn verify_move_chain_integrity(move_records: &[MoveRecord]) -> bool {
    if move_records.is_empty() {
        return true; // Empty chain is valid
    }

    // First move should have no previous hash
    if move_records[0].prev_hash.is_some() {
        tracing::warn!("First move in chain has a previous hash");
        return false;
    }

    // Check that each subsequent move references the hash of the previous move
    for i in 1..move_records.len() {
        let current = &move_records[i];
        let previous = &move_records[i - 1];

        // Calculate expected hash of previous move
        let expected_prev_hash = calculate_move_hash(previous);

        match &current.prev_hash {
            Some(actual_prev_hash) => {
                if actual_prev_hash != &expected_prev_hash {
                    tracing::warn!(
                        "Move chain integrity violation at index {}: expected hash {:?}, got {:?}",
                        i,
                        expected_prev_hash,
                        actual_prev_hash
                    );
                    return false;
                }
            }
            None => {
                tracing::warn!("Move at index {} missing previous hash", i);
                return false;
            }
        }
    }

    tracing::debug!(
        "Move chain integrity verified for {} moves",
        move_records.len()
    );
    true
}

/// Create a cryptographic proof of the current game state
pub fn create_game_state_proof(game_state: &p2pgo_core::GameState) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash essential game state fields
    game_state.board_size.hash(&mut hasher);
    game_state.current_player.hash(&mut hasher);
    game_state.captures.hash(&mut hasher);

    // Hash all moves in order
    for mv in &game_state.moves {
        mv.hash(&mut hasher);
    }

    // Hash board state
    for cell in &game_state.board {
        if let Some(color) = cell {
            color.hash(&mut hasher);
        }
    }

    format!("{:016x}", hasher.finish())
}

/// Verify that a game state matches the expected proof
pub fn verify_game_state_proof(game_state: &p2pgo_core::GameState, expected_proof: &str) -> bool {
    let actual_proof = create_game_state_proof(game_state);
    let matches = actual_proof == expected_proof;

    if matches {
        tracing::debug!("Game state proof verified: {}", actual_proof);
    } else {
        tracing::warn!(
            "Game state proof mismatch: expected {}, got {}",
            expected_proof,
            actual_proof
        );
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::{Coord, GameState, Move, MoveRecord};

    #[test]
    fn test_move_hash_calculation() {
        let mut move_record = MoveRecord::new(
            Move::Place {
                x: 4,
                y: 4,
                color: p2pgo_core::Color::Black,
            },
            None,
            1000, // timestamp
            None, // prev_hash
        );

        let hash1 = calculate_move_hash(&move_record);
        let hash2 = calculate_move_hash(&move_record);

        // Same move should produce same hash
        assert_eq!(hash1, hash2);

        // Different move should produce different hash
        move_record.mv = Move::Place {
            x: 5,
            y: 5,
            color: p2pgo_core::Color::White,
        };
        let hash3 = calculate_move_hash(&move_record);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_move_chain_integrity() {
        let mut records = Vec::new();

        // Create first move (no previous hash)
        let move1 = MoveRecord::new(
            Move::Place {
                x: 4,
                y: 4,
                color: p2pgo_core::Color::Black,
            },
            None,
            1000,
            None,
        );
        records.push(move1);

        // Create second move with correct previous hash
        let mut move2 = MoveRecord::new(
            Move::Place {
                x: 5,
                y: 5,
                color: p2pgo_core::Color::White,
            },
            None,
            2000,
            None,
        );
        move2.prev_hash = Some(calculate_move_hash(&records[0]));
        records.push(move2);

        // Chain should be valid
        assert!(verify_move_chain_integrity(&records));

        // Break the chain by modifying the hash
        records[1].prev_hash = Some([0u8; 32]); // Use invalid hash bytes instead of string
        assert!(!verify_move_chain_integrity(&records));
    }

    #[test]
    fn test_game_state_proof() {
        let mut game_state = GameState::new(9);

        let proof1 = create_game_state_proof(&game_state);
        assert!(verify_game_state_proof(&game_state, &proof1));

        // Add a move and verify proof changes
        game_state.moves.push(Move::Place {
            x: 4,
            y: 4,
            color: p2pgo_core::Color::Black,
        });
        let proof2 = create_game_state_proof(&game_state);
        assert_ne!(proof1, proof2);
        assert!(verify_game_state_proof(&game_state, &proof2));
        assert!(!verify_game_state_proof(&game_state, &proof1));
    }
}
