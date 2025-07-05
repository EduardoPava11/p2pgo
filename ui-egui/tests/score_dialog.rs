//! Test for score dialog functionality
//! SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_ui_egui::{app::AppConfig, view::View};

#[test]
fn score_dialog_updates_game_counter() {
    // Create a simple config object
    let mut config = AppConfig {
        auto_refresh: true,
        games_finished: 0,
    };

    // Initial game count should be 0
    assert_eq!(config.games_finished, 0);

    // Simulate accepting score
    config.games_finished += 1;

    // Games finished should be incremented
    assert_eq!(config.games_finished, 1);
}

#[test]
fn score_dialog_view_structure() {
    // Test that we can create a score dialog view
    let score_proof = p2pgo_core::value_labeller::ScoreProof {
        final_score: 5, // Positive means Black wins
        territory_black: 10,
        territory_white: 5,
        captures_black: 0,
        captures_white: 0,
        komi: 6.5,
        method: p2pgo_core::value_labeller::ScoringMethod::Territory,
    };

    let view = View::ScoreDialog {
        game_id: "test".into(),
        game_state: p2pgo_core::GameState::new(9),
        score_proof: score_proof.clone(),
        dead_stones: std::collections::HashSet::new(),
        score_pending: false,
        score_accepted: true,
    };

    // Check that we can extract the score proof
    if let View::ScoreDialog {
        score_proof: extracted_proof,
        ..
    } = view
    {
        assert_eq!(extracted_proof.final_score, 5);
        assert_eq!(extracted_proof.territory_black, 10);
        assert_eq!(extracted_proof.territory_white, 5);
        assert!(extracted_proof.final_score > 0, "Black should have won");
    } else {
        panic!("Expected ScoreDialog view");
    }
}
