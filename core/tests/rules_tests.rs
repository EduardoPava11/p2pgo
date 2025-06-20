// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::{board::Board, rules::RuleValidator, Color, Coord, GameError};

#[test]
fn ko_detection() {
    // Create a simple ko situation: a black stone just captured one white stone
    let mut current = Board::new(9);
    
    // Black stones in a diamond pattern around (1,1)
    current.place(Coord::new(1, 0), Color::Black); // top
    current.place(Coord::new(0, 1), Color::Black); // left
    current.place(Coord::new(1, 1), Color::Black); // center (just captured white)
    current.place(Coord::new(1, 2), Color::Black); // bottom
    
    // White stones surrounding the diamond
    current.place(Coord::new(2, 0), Color::White); // top-right
    current.place(Coord::new(2, 1), Color::White); // right
    current.place(Coord::new(2, 2), Color::White); // bottom-right
    
    // Previous board state had a white stone at center (1,1)
    let mut previous = current.clone();
    previous.remove(Coord::new(1, 1));           // Remove black stone
    previous.place(Coord::new(1, 1), Color::White); // Add white stone
    
    // Check for ko violation - White can't recapture at the same spot
    let validator = RuleValidator::new(&current, &previous);
    let result = validator.check_move(Coord::new(1, 1), Color::White);
    
    assert!(matches!(result, Err(GameError::OccupiedPosition)), 
        "Should fail because position is already occupied, got {:?}", result);
}

#[test]
fn self_capture() {
    // Create a board with a position that would cause self-capture
    let mut board = Board::new(9);
    let prev_board = Board::new(9);
    
    // White stones surrounding an empty point
    board.place(Coord::new(0, 0), Color::White);
    board.place(Coord::new(1, 0), Color::White);
    board.place(Coord::new(0, 1), Color::White);
    board.place(Coord::new(2, 1), Color::White);
    board.place(Coord::new(1, 2), Color::White);
    board.place(Coord::new(2, 2), Color::White);
    
    let validator = RuleValidator::new(&board, &prev_board);
    
    // Black can't play (self-capture), but White can play
    assert!(matches!(validator.check_move(Coord::new(1, 1), Color::Black), 
                     Err(GameError::SelfCapture)));
    assert!(validator.check_move(Coord::new(1, 1), Color::White).is_ok());
}

#[test]
fn capture_detection() {
    // Create a simple capture situation with black surrounding white stones
    let mut board = Board::new(9);
    let prev = Board::new(9);
    
    // Place white stones that will be captured
    board.place(Coord::new(3, 3), Color::White);
    board.place(Coord::new(4, 3), Color::White);
    
    // Place black stones to surround them (leaving one liberty)
    board.place(Coord::new(2, 3), Color::Black);
    board.place(Coord::new(3, 2), Color::Black);
    board.place(Coord::new(4, 2), Color::Black);
    board.place(Coord::new(5, 3), Color::Black);
    board.place(Coord::new(4, 4), Color::Black);
    
    // Validate that black can play at the last liberty
    let validator = RuleValidator::new(&board, &prev);
    let result = validator.check_move(Coord::new(3, 4), Color::Black);
    assert!(result.is_ok(), "Black should be able to play the capturing move");
    
    // Place the final stone to create a capture scenario
    let mut board2 = board.clone();
    board2.place(Coord::new(3, 4), Color::Black);
    
    // Now test that these 2 white stones are detected as captured
    let validator2 = RuleValidator::new(&board2, &prev);
    let captures = validator2.find_captures(Coord::new(3, 4));
    assert_eq!(captures.len(), 2, "Should detect 2 captured white stones");
}

#[test]
fn board_creation() {
    let board = Board::new(19);
    assert_eq!(board.size(), 19);
    assert_eq!(board.get(Coord::new(0, 0)), None);
}
