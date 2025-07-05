use p2pgo_core::*;
use std::collections::HashSet;

#[cfg(test)]
mod comprehensive_core_tests {
    use super::*;

    // Test 1: Complete game lifecycle
    #[test]
    fn test_full_game_lifecycle() {
        let mut game = GameState::new(9);
        
        // Opening moves
        assert!(game.apply_move(Move::Place { x: 3, y: 3, color: Color::Black }).is_ok());
        assert!(game.apply_move(Move::Place { x: 5, y: 5, color: Color::White }).is_ok());
        assert!(game.apply_move(Move::Place { x: 3, y: 5, color: Color::Black }).is_ok());
        assert!(game.apply_move(Move::Place { x: 5, y: 3, color: Color::White }).is_ok());
        
        // Test capture
        assert!(game.apply_move(Move::Place { x: 4, y: 5, color: Color::Black }).is_ok());
        assert!(game.apply_move(Move::Place { x: 6, y: 5, color: Color::White }).is_ok());
        assert!(game.apply_move(Move::Place { x: 5, y: 4, color: Color::Black }).is_ok());
        assert!(game.apply_move(Move::Place { x: 5, y: 6, color: Color::White }).is_ok());
        
        // This should capture the white stone
        assert!(game.apply_move(Move::Place { x: 5, y: 6, color: Color::Black }).is_err()); // Already occupied
        assert!(game.apply_move(Move::Place { x: 6, y: 4, color: Color::Black }).is_ok());
        
        // Verify capture happened
        assert_eq!(game.prisoners.0, 0); // Black prisoners
        assert_eq!(game.prisoners.1, 1); // White prisoners (captured)
        
        // End game with passes
        assert!(game.apply_move(Move::Pass).is_ok());
        assert!(game.apply_move(Move::Pass).is_ok());
        
        assert!(game.is_game_over());
    }

    // Test 2: Ko rule enforcement
    #[test]
    fn test_ko_rule() {
        let mut game = GameState::new(9);
        
        // Set up ko situation
        // B W .
        // W . W
        // . W .
        game.apply_move(Move::Place { x: 0, y: 0, color: Color::Black }).unwrap();
        game.apply_move(Move::Place { x: 1, y: 0, color: Color::White }).unwrap();
        game.apply_move(Move::Place { x: 2, y: 2, color: Color::Black }).unwrap(); // Dummy
        game.apply_move(Move::Place { x: 0, y: 1, color: Color::White }).unwrap();
        game.apply_move(Move::Place { x: 3, y: 3, color: Color::Black }).unwrap(); // Dummy
        game.apply_move(Move::Place { x: 2, y: 1, color: Color::White }).unwrap();
        game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }).unwrap(); // Dummy
        game.apply_move(Move::Place { x: 1, y: 2, color: Color::White }).unwrap();
        
        // Black captures at (1,1)
        assert!(game.apply_move(Move::Place { x: 1, y: 1, color: Color::Black }).is_ok());
        
        // White cannot immediately recapture (ko rule)
        assert!(game.apply_move(Move::Place { x: 1, y: 0, color: Color::White }).is_err());
        
        // White must play elsewhere
        assert!(game.apply_move(Move::Place { x: 7, y: 7, color: Color::White }).is_ok());
        
        // Now black plays elsewhere
        assert!(game.apply_move(Move::Place { x: 6, y: 6, color: Color::Black }).is_ok());
        
        // Now white can recapture (ko cleared)
        assert!(game.apply_move(Move::Place { x: 1, y: 0, color: Color::White }).is_ok());
    }

    // Test 3: Score calculation with territory
    #[test]
    fn test_score_calculation() {
        let mut game = GameState::new(9);
        
        // Create simple territories
        // Black controls top-left
        for x in 0..3 {
            for y in 0..3 {
                if x == 2 || y == 2 {
                    game.apply_move(Move::Place { x, y, color: Color::Black }).unwrap();
                    game.apply_move(Move::Pass).unwrap(); // White passes
                }
            }
        }
        
        // White controls bottom-right
        for x in 6..9 {
            for y in 6..9 {
                if x == 6 || y == 6 {
                    game.apply_move(Move::Pass).unwrap(); // Black passes
                    game.apply_move(Move::Place { x, y, color: Color::White }).unwrap();
                }
            }
        }
        
        // End game
        game.apply_move(Move::Pass).unwrap();
        game.apply_move(Move::Pass).unwrap();
        
        // Calculate score
        let score_proof = scoring::calculate_final_score(
            &game,
            5.5, // komi
            value_labeller::ScoringMethod::Territory,
            &HashSet::new(), // no dead stones
        );
        
        println!("Black territory: {}", score_proof.territory_black);
        println!("White territory: {}", score_proof.territory_white);
        println!("Final score: {}", score_proof.final_score);
        
        assert!(score_proof.territory_black > 0);
        assert!(score_proof.territory_white > 0);
    }

    // Test 4: CBOR archiving
    #[test]
    fn test_cbor_archiving() {
        use tempfile::tempdir;
        
        let mut game = GameState::new(9);
        
        // Play some moves
        game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }).unwrap();
        game.apply_move(Move::Place { x: 4, y: 5, color: Color::White }).unwrap();
        game.apply_move(Move::Pass).unwrap();
        game.apply_move(Move::Pass).unwrap();
        
        // Archive game
        let temp_dir = tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        
        let archive_path = archiver::archive_finished_game(&game, "test_opponent").unwrap();
        assert!(archive_path.exists());
        
        // Read back
        let restored = archiver::read_game_archive(&archive_path).unwrap();
        assert_eq!(restored.moves.len(), game.moves.len());
        assert_eq!(restored.board_size, game.board_size);
        assert_eq!(restored.prisoners, game.prisoners);
    }

    // Test 5: SGF parsing
    #[test]
    fn test_sgf_parsing() {
        let sgf_content = r#"(;GM[1]FF[4]CA[UTF-8]SZ[9]KM[5.5]
PB[Black Player]PW[White Player]
;B[ee];W[eg];B[dd];W[dg])"#;
        
        let games = sgf::parse_sgf(sgf_content).unwrap();
        assert_eq!(games.len(), 1);
        
        let game = &games[0];
        assert_eq!(game.board_size, 9);
        assert_eq!(game.komi, 5.5);
        assert_eq!(game.moves.len(), 4);
        
        // Convert to training record
        let record = sgf::sgf_to_training_record(&games[0]).unwrap();
        assert_eq!(record.positions.len(), 4);
    }

    // Test 6: Move validation edge cases
    #[test]
    fn test_move_validation() {
        let mut game = GameState::new(9);
        
        // Out of bounds
        assert!(game.apply_move(Move::Place { x: 9, y: 0, color: Color::Black }).is_err());
        assert!(game.apply_move(Move::Place { x: 0, y: 9, color: Color::Black }).is_err());
        
        // Occupied position
        game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }).unwrap();
        assert!(game.apply_move(Move::Place { x: 4, y: 4, color: Color::White }).is_err());
        
        // Wrong turn
        let wrong_move = Move::Place { x: 5, y: 5, color: Color::Black };
        assert!(game.apply_move(wrong_move).is_err());
        
        // Suicide rule
        let mut suicide_game = GameState::new(9);
        // Surround a point with opponent stones
        suicide_game.apply_move(Move::Place { x: 1, y: 0, color: Color::Black }).unwrap();
        suicide_game.apply_move(Move::Place { x: 0, y: 1, color: Color::White }).unwrap();
        suicide_game.apply_move(Move::Place { x: 2, y: 1, color: Color::Black }).unwrap();
        suicide_game.apply_move(Move::Place { x: 1, y: 2, color: Color::White }).unwrap();
        suicide_game.apply_move(Move::Pass).unwrap(); // Black pass
        
        // White suicide at (1,1) should fail
        assert!(suicide_game.apply_move(Move::Place { x: 1, y: 1, color: Color::White }).is_err());
    }

    // Test 7: Large game compression
    #[test]
    fn test_large_game_compression() {
        use tempfile::tempdir;
        
        let mut game = GameState::new(19);
        
        // Fill board with many moves to exceed 1MB threshold
        for i in 0..200 {
            let x = (i * 7) % 19;
            let y = (i * 11) % 19;
            let color = if i % 2 == 0 { Color::Black } else { Color::White };
            
            if game.get_stone(x as u8, y as u8).is_none() {
                let _ = game.apply_move(Move::Place { x: x as u8, y: y as u8, color });
            } else {
                let _ = game.apply_move(Move::Pass);
            }
        }
        
        // Archive
        let temp_dir = tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        
        let archive_path = archiver::archive_finished_game(&game, "compression_test").unwrap();
        
        // Should be compressed
        if game.moves.len() > 100 {
            assert!(archive_path.to_string_lossy().ends_with(".cbor.gz"));
        }
        
        // Verify can read back
        let restored = archiver::read_game_archive(&archive_path).unwrap();
        assert_eq!(restored.moves.len(), game.moves.len());
    }
}