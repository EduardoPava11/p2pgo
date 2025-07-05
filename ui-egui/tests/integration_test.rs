// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests to verify the UI compiles correctly
//!
//! Since the UI is a binary crate, we can't directly test internal modules,
//! but we can verify that our fixes don't break compilation.

// AI trainer integration tests - simplified since NetworkWorker is private
use p2pgo_core::{Color, GameState};
use p2pgo_ui_egui::msg::{NetToUi, UiToNet};

#[test]
fn test_compilation() {
    // This test simply ensures that the code compiles correctly.
    // The real test is that the binary can be built without errors.
    println!("Basic compilation test passed");
}

#[test]
fn test_ai_integration_messages() {
    // Test message enum has GetGhostMoves variant
    let _msg = UiToNet::GetGhostMoves;

    // Test GhostMoves response variant exists (tuple variant)
    let _response = NetToUi::GhostMoves(vec![]);

    println!("AI integration message types compile successfully");
}

#[test]
fn test_game_state_creation() {
    // Create a simple game state for testing
    let game_state = GameState::new(9);

    assert_eq!(game_state.board_size, 9);
    assert_eq!(game_state.board.len(), 81);
    assert_eq!(game_state.current_player, Color::Black);
    assert_eq!(game_state.moves.len(), 0);
    assert_eq!(game_state.captures, (0, 0));

    println!("GameState creation works correctly");
}

#[test]
fn test_tensor_conversion_concept() {
    // Test the concept of converting board state to tensor format
    let mut board = vec![None; 81]; // 9x9 board
    board[0] = Some(Color::Black);
    board[1] = Some(Color::White);

    // Convert to tensor manually (same logic as in NetworkWorker::game_state_to_tensor)
    let mut tensor = vec![0.0f32; 81];
    for (i, &stone) in board.iter().enumerate() {
        tensor[i] = match stone {
            Some(Color::Black) => 1.0,
            Some(Color::White) => -1.0,
            None => 0.0,
        };
    }

    assert_eq!(
        tensor.len(),
        81,
        "Tensor should have 81 elements for 9x9 board"
    );
    assert_eq!(tensor[0], 1.0, "First position should be Black (1.0)");
    assert_eq!(tensor[1], -1.0, "Second position should be White (-1.0)");
    assert_eq!(tensor[2], 0.0, "Third position should be empty (0.0)");

    println!("Tensor conversion concept works correctly");
}
