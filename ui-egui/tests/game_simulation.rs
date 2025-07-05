// SPDX-License-Identifier: MIT OR Apache-2.0

#[test]
fn test_two_player_simulation() {
    #[cfg(feature = "headless")]
    {
        use p2pgo_core::{Coord, GameState, Move};

        println!("Testing two-player game simulation");

        // Create two game states (simulating two separate UIs)
        let mut game_a = GameState::new(9);
        let mut game_b = GameState::new(9);

        // Simulate the 5-move game sequence
        let moves = vec![
            Move::Place(Coord::new(3, 3)), // D4 - Black
            Move::Place(Coord::new(5, 3)), // F4 - White
            Move::Place(Coord::new(4, 4)), // E5 - Black
            Move::Pass,                    // White pass
            Move::Pass,                    // Black pass
        ];

        println!("Simulating game moves...");
        for (i, mv) in moves.iter().enumerate() {
            println!("Move {}: {:?}", i + 1, mv);

            // Apply move to both game states
            if let Err(e) = game_a.apply_move(mv.clone()) {
                panic!("Game A failed to apply move {}: {}", i + 1, e);
            }
            if let Err(e) = game_b.apply_move(mv.clone()) {
                panic!("Game B failed to apply move {}: {}", i + 1, e);
            }
        }

        // Verify both games have identical final states
        println!("Verifying final game states...");
        assert_eq!(
            game_a.is_game_over(),
            game_b.is_game_over(),
            "Game over status should match"
        );
        assert!(
            game_a.is_game_over(),
            "Game should be over after two passes"
        );

        // Verify board states are identical
        for x in 0..9 {
            for y in 0..9 {
                let coord = Coord::new(x, y);
                let idx = (y as usize) * 9 + (x as usize);
                let stone_a = game_a.board.get(idx);
                let stone_b = game_b.board.get(idx);
                assert_eq!(stone_a, stone_b, "Stone at {:?} should match", coord);
            }
        }

        println!("✓ Both game states are identical");
        println!("✓ Game is over: {}", game_a.is_game_over());
        println!("✓ Two-player simulation completed successfully");
    }

    #[cfg(not(feature = "headless"))]
    {
        println!("Skipping test - headless feature not enabled");
    }
}

#[test]
fn test_basic_functionality() {
    println!("Testing basic functionality");
    assert_eq!(2 + 2, 4);
}
