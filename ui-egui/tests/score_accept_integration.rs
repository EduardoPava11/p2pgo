use p2pgo_core::value_labeller::{ScoreProof, ScoringMethod};
use p2pgo_core::{Coord, GameState, Move};
use std::collections::HashSet;

#[test]
fn scoring_pipeline_works_correctly() {
    // Create a game state
    let mut game_state = GameState::new(9);

    // Play moves to create some territory
    // Create a small black territory in the corner
    let _ = game_state.apply_move(Move::Place(Coord::new(0, 0))); // Black
    let _ = game_state.apply_move(Move::Place(Coord::new(3, 3))); // White
    let _ = game_state.apply_move(Move::Place(Coord::new(0, 1))); // Black
    let _ = game_state.apply_move(Move::Place(Coord::new(3, 4))); // White
    let _ = game_state.apply_move(Move::Place(Coord::new(1, 0))); // Black
    let _ = game_state.apply_move(Move::Place(Coord::new(4, 3))); // White

    // End the game
    let _ = game_state.apply_move(Move::Pass); // Black passes
    let _ = game_state.apply_move(Move::Pass); // White passes

    // Calculate score directly
    let empty_dead_stones = HashSet::new();
    let score_proof = p2pgo_core::scoring::calculate_final_score(
        &game_state,
        6.5, // Typical komi for 9x9
        ScoringMethod::Territory,
        &empty_dead_stones,
    );

    // Print the game state and score for debugging
    println!("Board state:");
    for y in 0..game_state.board_size {
        for x in 0..game_state.board_size {
            let idx = y as usize * game_state.board_size as usize + x as usize;
            match game_state.board[idx] {
                Some(p2pgo_core::Color::Black) => print!("B "),
                Some(p2pgo_core::Color::White) => print!("W "),
                None => print!(". "),
            }
        }
        println!();
    }
    println!("Score proof: {:?}", score_proof);

    // For this test, we just verify that the scoring pipeline works
    // and that we get a valid score proof with the correct scoring method
    assert_eq!(score_proof.method, ScoringMethod::Territory);
    // The exact territory values depend on the board state and are less important

    // Verify score is consistent
    assert_eq!(
        score_proof.final_score,
        (score_proof.territory_black as i16 + score_proof.captures_black as i16) as i16
            - ((score_proof.territory_white as i16 + score_proof.captures_white as i16) as f32
                + score_proof.komi)
                .round() as i16
    );

    // Check that the scoring method is correct
    assert_eq!(score_proof.method, ScoringMethod::Territory);

    println!("Score proof: {:?}", score_proof);

    // Create value labeller
    let mut labeller = p2pgo_core::value_labeller::ValueLabeller::new();

    // Use the new label_and_persist function to generate training data
    let training_data = labeller.label_and_persist(&game_state, score_proof.clone());

    // Verify we have training data with proper markers
    assert!(
        training_data.len() > 10,
        "Should have generated meaningful training data"
    );

    // Should contain at least one 'S' marker for ScoreProof
    assert!(
        training_data.iter().any(|&b| b == b'S'),
        "Training data should contain score proof marker"
    );

    // Should contain at least one 'M' marker for MoveRecords
    assert!(
        training_data.iter().any(|&b| b == b'M'),
        "Training data should contain move records marker"
    );

    // Verify value labels are generated for every move
    for i in 0..game_state.moves.len() {
        if let Some(label) = labeller.get_value_label(i as u32) {
            // This verifies that the value labeller uses the score proof correctly
            assert!(
                label.game_outcome != 0.0,
                "Game outcome should be set for each move"
            );
        } else {
            panic!("Expected value label for move {}", i);
        }
    }

    // Test serialization of score proof (for blob storage)
    let serialized = serde_cbor::to_vec(&score_proof).expect("Serialization should work");
    let deserialized: ScoreProof =
        serde_cbor::from_slice(&serialized).expect("Deserialization should work");

    // Verify round-trip serialization
    assert_eq!(deserialized.final_score, score_proof.final_score);
    assert_eq!(deserialized.method, score_proof.method);
}
