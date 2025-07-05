// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::{Coord, GameState, Move};
use p2pgo_network::blob_store::{MoveBlob, MoveChain};

#[test]
fn chain_accepts_sequential_moves() {
    let gid = "game-demo".to_string();
    let mut chain = MoveChain::new(gid.clone());

    // Create initial state and apply first move
    let mut gs0 = GameState::new(9); // empty
    gs0.apply_move(Move::Pass).unwrap(); // Apply first pass
    let b0 = MoveBlob::new(gid.clone(), Move::Pass, None, gs0.clone(), 0);
    chain.add_blob(b0.clone()).unwrap();

    // Apply second move to get the next state
    let mut gs1 = gs0.clone();
    gs1.apply_move(Move::Pass).unwrap(); // Apply second pass
    let b1 = MoveBlob::new(gid.clone(), Move::Pass, Some(b0.hash()), gs1, 1);
    assert!(chain.add_blob(b1).is_ok());
    assert_eq!(chain.current_sequence, 1);
}

#[test]
fn chain_rejects_wrong_game_id() {
    let gid1 = "game-one".to_string();
    let gid2 = "game-two".to_string();
    let mut chain = MoveChain::new(gid1.clone());

    let gs = GameState::new(9);
    let blob = MoveBlob::new(gid2, Move::Pass, None, gs, 0);

    let result = chain.add_blob(blob);
    assert!(result.is_err());
}

#[test]
fn chain_rejects_wrong_sequence() {
    let gid = "game-seq".to_string();
    let mut chain = MoveChain::new(gid.clone());

    let gs = GameState::new(9);
    let blob = MoveBlob::new(gid.clone(), Move::Pass, None, gs.clone(), 1); // Should be 0

    let result = chain.add_blob(blob);
    assert!(result.is_err());

    // Now add a correct blob and try to add another with wrong sequence
    let mut gs0 = gs.clone();
    gs0.apply_move(Move::Pass).unwrap();
    let b0 = MoveBlob::new(gid.clone(), Move::Pass, None, gs0.clone(), 0);
    chain.add_blob(b0.clone()).unwrap();

    // Trying to add with sequence 2 (skipping 1) - use proper state
    let mut gs2 = gs0.clone();
    gs2.apply_move(Move::Pass).unwrap();
    let b2 = MoveBlob::new(gid.clone(), Move::Pass, Some(b0.hash()), gs2, 2);
    let result = chain.add_blob(b2);
    assert!(result.is_err());
}

#[test]
fn chain_maintains_blob_order() {
    let gid = "game-order".to_string();
    let mut chain = MoveChain::new(gid.clone());

    // Apply first pass
    let mut gs0 = GameState::new(9);
    gs0.apply_move(Move::Pass).unwrap();
    let b0 = MoveBlob::new(gid.clone(), Move::Pass, None, gs0.clone(), 0);
    chain.add_blob(b0.clone()).unwrap();

    // Apply a move to create a new game state
    let mut gs1 = gs0.clone();
    gs1.apply_move(Move::Place(Coord::new(4, 4))).unwrap();

    let b1 = MoveBlob::new(
        gid.clone(),
        Move::Place(Coord::new(4, 4)),
        Some(b0.hash()),
        gs1,
        1,
    );
    chain.add_blob(b1.clone()).unwrap();

    // Get all blobs and check order
    let blobs = chain.get_all_blobs();
    assert_eq!(blobs.len(), 2);
    assert_eq!(blobs[0].sequence, 0);
    assert_eq!(blobs[1].sequence, 1);
}
