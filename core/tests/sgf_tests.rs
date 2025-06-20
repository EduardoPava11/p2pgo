// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::{GameState, Move, Coord};
use p2pgo_core::sgf::SgfProcessor;

#[test]
fn sgf_roundtrip() {
    let mut gs = GameState::new(9);
    gs.apply_move(Move::Place(Coord::new(0, 0))).unwrap();
    gs.apply_move(Move::Place(Coord::new(1, 0))).unwrap();
    let sgf = SgfProcessor::new(gs.clone()).generate();
    let mut processor = SgfProcessor::new(GameState::new(9));
    let parsed = processor.parse(&sgf).unwrap();
    assert_eq!(parsed.moves.len(), 2);
}

#[test]
fn parse_simple_sgf() {
    // Simple SGF string
    let sgf = "(;GM[1]FF[4]SZ[9];B[ee];W[dc];B[fc];W[];B[hh])";
    
    let mut processor = SgfProcessor::new(GameState::new(9));
    let game_state = processor.parse(sgf).unwrap();
    
    assert_eq!(game_state.board_size, 9);
    assert_eq!(game_state.moves.len(), 5);
}
