// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::{GameState, Move, Coord};

#[test]
fn test_move_sequence_order() {
    // Create a new game state
    let mut state1 = GameState::new(9);
    
    // Apply moves in sequence 1
    let _ = state1.apply_move(Move::Place(Coord::new(2, 3)));
    let _ = state1.apply_move(Move::Place(Coord::new(3, 3)));
    let _ = state1.apply_move(Move::Place(Coord::new(2, 4)));
    
    // Get the board hash after sequence 1
    let hash1 = state1.board.position_hash();
    
    // Create a second game state
    let mut state2 = GameState::new(9);
    
    // Apply the same moves in a different order
    let _ = state2.apply_move(Move::Place(Coord::new(2, 3)));
    let _ = state2.apply_move(Move::Place(Coord::new(2, 4)));
    let _ = state2.apply_move(Move::Place(Coord::new(3, 3)));
    
    // Get the board hash after sequence 2
    let hash2 = state2.board.position_hash();
    
    // Different move orders should result in different boards
    assert_ne!(hash1, hash2, "Different move orders should result in different boards");
    
    // Create sequences that should produce identical positions
    let mut state3 = GameState::new(9);
    state3.apply_move(Move::Place(Coord::new(1, 1))).unwrap();
    state3.apply_move(Move::Place(Coord::new(7, 7))).unwrap();
    
    let mut state4 = GameState::new(9);
    state4.apply_move(Move::Place(Coord::new(1, 1))).unwrap();
    state4.apply_move(Move::Place(Coord::new(7, 7))).unwrap();
    
    // These should be the same since the moves are identical
    let hash3 = state3.board.position_hash();
    let hash4 = state4.board.position_hash();
    
    assert_eq!(hash3, hash4, "Identical move sequences should result in identical boards");
    
    println!("Basic move ordering test passed!");
}
