// Direct test of core functionality
use p2pgo_core::*;
use std::collections::HashSet;

fn main() {
    println!("🧪 Testing Core Module Functionality\n");

    // Test 1: Basic game flow
    println!("Test 1: Basic Game Flow");
    let mut game = GameState::new(9);
    
    match game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }) {
        Ok(_) => println!("✅ Black played at (4,4)"),
        Err(e) => println!("❌ Move failed: {}", e),
    }
    
    match game.apply_move(Move::Place { x: 5, y: 5, color: Color::White }) {
        Ok(_) => println!("✅ White played at (5,5)"),
        Err(e) => println!("❌ Move failed: {}", e),
    }
    
    println!("  Current player: {:?}", game.current_player);
    println!("  Moves made: {}", game.moves.len());
    
    // Test 2: Ko rule
    println!("\nTest 2: Ko Rule");
    let mut ko_game = GameState::new(9);
    
    // Setup ko situation
    let setup_moves = vec![
        (0, 0, Color::Black),
        (1, 0, Color::White),
        (0, 1, Color::Black),
        (0, 2, Color::White),
        (1, 1, Color::Black),
        (2, 1, Color::White),
        (1, 2, Color::Black),
        (1, 3, Color::White),
    ];
    
    for (x, y, color) in setup_moves {
        let _ = ko_game.apply_move(Move::Place { x, y, color });
    }
    
    // This captures white stone at (1,0)
    match ko_game.apply_move(Move::Place { x: 2, y: 0, color: Color::Black }) {
        Ok(_) => println!("✅ Black captures at (2,0)"),
        Err(e) => println!("❌ Capture failed: {}", e),
    }
    
    // White tries to recapture immediately (ko violation)
    match ko_game.apply_move(Move::Place { x: 1, y: 0, color: Color::White }) {
        Ok(_) => println!("❌ Ko rule not enforced!"),
        Err(_) => println!("✅ Ko rule correctly prevented immediate recapture"),
    }
    
    // Test 3: Score calculation
    println!("\nTest 3: Score Calculation");
    let mut score_game = GameState::new(9);
    
    // Create simple territories
    for i in 0..5 {
        let _ = score_game.apply_move(Move::Place { x: i, y: 0, color: Color::Black });
        let _ = score_game.apply_move(Move::Place { x: i, y: 8, color: Color::White });
    }
    
    // End game
    let _ = score_game.apply_move(Move::Pass);
    let _ = score_game.apply_move(Move::Pass);
    
    if score_game.is_game_over() {
        println!("✅ Game correctly ended after two passes");
        
        let score_proof = scoring::calculate_final_score(
            &score_game,
            5.5,
            value_labeller::ScoringMethod::Territory,
            &HashSet::new(),
        );
        
        println!("  Black territory: {}", score_proof.territory_black);
        println!("  White territory: {}", score_proof.territory_white);
        println!("  Final score: {}", score_proof.final_score);
    }
    
    // Test 4: CBOR archiving
    println!("\nTest 4: CBOR Archiving");
    use std::env;
    use tempfile::tempdir;
    
    let temp_dir = tempdir().unwrap();
    env::set_var("HOME", temp_dir.path());
    
    match archiver::archive_finished_game(&score_game, "test_opponent") {
        Ok(path) => {
            println!("✅ Game archived to: {:?}", path.file_name().unwrap());
            
            // Try to read it back
            match archiver::read_game_archive(&path) {
                Ok(restored) => {
                    println!("✅ Archive read successfully");
                    println!("  Restored moves: {}", restored.moves.len());
                },
                Err(e) => println!("❌ Failed to read archive: {}", e),
            }
        },
        Err(e) => println!("❌ Failed to archive: {}", e),
    }
    
    // Test 5: SGF parsing
    println!("\nTest 5: SGF Parsing");
    let sgf = "(;GM[1]FF[4]SZ[9]KM[5.5];B[ee];W[eg];B[dd];W[dg])";
    
    match sgf::parse_sgf(sgf) {
        Ok(games) => {
            println!("✅ SGF parsed successfully");
            if let Some(game) = games.first() {
                println!("  Board size: {}", game.board_size);
                println!("  Komi: {}", game.komi);
                println!("  Moves: {}", game.moves.len());
                
                // Convert to training record
                match sgf::sgf_to_training_record(game) {
                    Ok(record) => println!("✅ Converted to training record with {} positions", record.positions.len()),
                    Err(e) => println!("❌ Training conversion failed: {}", e),
                }
            }
        },
        Err(e) => println!("❌ SGF parsing failed: {}", e),
    }
    
    println!("\n🎯 Core module tests completed!");
}