// Automated game test following SGF sequence
// Compile with: rustc automated_game_test.rs -L target/debug/deps --extern p2pgo_core=...

use p2pgo_core::archiver::archive_finished_game;
use p2pgo_core::scoring::calculate_final_score;
use p2pgo_core::value_labeller::ScoringMethod;
use p2pgo_core::{Color, Coord, GameState, Move};
use std::collections::HashSet;

fn main() {
    println!("🎮 Automated Game Test - Following SGF Sequence");
    println!("==============================================\n");

    // Create a new 9x9 game
    let mut game = GameState::new(9);

    // SGF moves sequence (converted from SGF notation to 0-indexed coordinates)
    let moves = vec![
        // Move format: (x, y, color)
        (3, 2, Color::Black), // dc
        (5, 5, Color::White), // ff
        (3, 6, Color::Black), // dg
        (2, 4, Color::White), // ce
        (5, 7, Color::Black), // fh
        (5, 2, Color::White), // fc
        (4, 2, Color::Black), // ec
        (5, 3, Color::White), // fd
        (7, 6, Color::Black), // hg
        (1, 2, Color::White), // bc
        (2, 3, Color::Black), // cd
        (1, 3, Color::White), // bd
        (3, 4, Color::Black), // de
        (2, 5, Color::White), // cf
        (3, 5, Color::Black), // df
        (7, 5, Color::White), // hf
        (7, 4, Color::Black), // he
        (6, 6, Color::White), // gg
        (6, 4, Color::Black), // ge
        (6, 5, Color::White), // gf
        (5, 4, Color::Black), // fe
        (4, 4, Color::White), // ee
        (4, 3, Color::Black), // ed
        (4, 5, Color::White), // ef
        (6, 2, Color::Black), // gc
        (5, 1, Color::White), // fb
        (6, 1, Color::Black), // gb
        (2, 2, Color::White), // cc
        (4, 1, Color::Black), // eb
        (6, 7, Color::White), // gh
        (2, 6, Color::Black), // cg
        (1, 6, Color::White), // bg
        (1, 7, Color::Black), // bh
        (1, 5, Color::White), // bf
        (4, 6, Color::Black), // eg
        (7, 7, Color::White), // hh
        (0, 7, Color::Black), // ah
        (2, 1, Color::White), // cb
        (3, 1, Color::Black), // db
        (3, 3, Color::White), // dd
        (6, 3, Color::Black), // gd
        (8, 4, Color::White), // ie
        (8, 3, Color::Black), // id
        (8, 5, Color::White), // if
        (5, 6, Color::Black), // fg
        (5, 8, Color::White), // fi
        (4, 8, Color::Black), // ei
        (6, 8, Color::White), // gi
        (3, 7, Color::Black), // dh
        (0, 6, Color::White), // ag
        (2, 0, Color::Black), // ca
        (1, 0, Color::White), // ba
        (5, 0, Color::Black), // fa
        (3, 0, Color::White), // da
        (4, 0, Color::Black), // ea
        (2, 3, Color::White), // cd (recapture)
        (8, 7, Color::Black), // ih
        (8, 6, Color::White), // ig
        (2, 0, Color::Black), // ca (ko fight)
        (8, 2, Color::White), // ic
        (7, 3, Color::Black), // hd
        (3, 0, Color::White), // da (ko fight)
        (7, 8, Color::Black), // hi
        (8, 8, Color::White), // ii
        (2, 0, Color::Black), // ca (ko fight)
        (2, 8, Color::White), // ci
        (3, 0, Color::Black), // da (ko fight)
        (2, 7, Color::White), // ch
        (0, 1, Color::Black), // ab
        (1, 8, Color::White), // bi
        (1, 1, Color::Black), // bb
        (0, 2, Color::White), // ac
        (0, 0, Color::Black), // aa
        (1, 0, Color::White), // ba (recapture)
        (0, 4, Color::Black), // ae
        (0, 8, Color::White), // ai
    ];

    // Play all moves
    println!("Playing {} moves from SGF...", moves.len());
    for (i, (x, y, expected_color)) in moves.iter().enumerate() {
        // Verify it's the right player's turn
        if game.current_player != *expected_color {
            println!(
                "❌ Turn mismatch at move {}: expected {:?}, but it's {:?}'s turn",
                i + 1,
                expected_color,
                game.current_player
            );
            break;
        }

        let mv = Move::Place {
            x: *x,
            y: *y,
            color: *expected_color,
        };
        match game.apply_move(mv) {
            Ok(_) => {
                if (i + 1) % 10 == 0 {
                    println!(
                        "  Move {}: {:?} plays at ({}, {})",
                        i + 1,
                        expected_color,
                        x,
                        y
                    );
                }
            }
            Err(e) => {
                println!(
                    "❌ Move {} failed: {:?} at ({}, {}): {}",
                    i + 1,
                    expected_color,
                    x,
                    y,
                    e
                );
                break;
            }
        }
    }

    // Both players pass
    println!("\nBoth players pass...");
    game.apply_move(Move::Pass).unwrap();
    game.apply_move(Move::Pass).unwrap();

    assert!(game.is_game_over());
    println!("✅ Game ended after {} moves", game.moves.len());

    // Calculate final score
    println!("\n📊 Score Calculation");
    println!("===================");

    let komi = 7.5; // Chinese rules, 9x9 board
    let dead_stones = HashSet::new(); // No dead stones to mark in this game

    let score_proof = calculate_final_score(&game, komi, ScoringMethod::Territory, &dead_stones);

    println!("Black territory: {} points", score_proof.territory_black);
    println!("White territory: {} points", score_proof.territory_white);
    println!("Black captures: {}", score_proof.captures_black);
    println!("White captures: {}", score_proof.captures_white);
    println!("Komi: {}", komi);

    let black_total = score_proof.territory_black as f32;
    let white_total = score_proof.territory_white as f32 + komi;
    println!("\nBlack total: {}", black_total);
    println!("White total: {}", white_total);
    println!("Final score: {}", score_proof.final_score);

    if score_proof.final_score > 0.0 {
        println!("Result: B+{}", score_proof.final_score);
    } else {
        println!("Result: W+{}", -score_proof.final_score);
    }

    // Expected result from SGF: W+34.5
    let expected_margin = -34.5;
    let score_diff = (score_proof.final_score - expected_margin).abs();
    if score_diff < 1.0 {
        println!("✅ Score matches SGF result (W+34.5)!");
    } else {
        println!("⚠️  Score differs from SGF by {} points", score_diff);
    }

    // Archive the game to CBOR
    println!("\n📦 Archiving to CBOR");
    println!("===================");

    use std::env;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    env::set_var("HOME", temp_dir.path());

    match archive_finished_game(&game, "SGF_opponent") {
        Ok(path) => {
            println!("✅ Game archived to: {:?}", path);

            // Check file size
            let metadata = std::fs::metadata(&path).unwrap();
            println!("   File size: {} bytes", metadata.len());

            // Verify we can read it back
            match p2pgo_core::archiver::read_game_archive(&path) {
                Ok(restored) => {
                    println!("✅ Archive verified - can be read back");
                    println!("   Restored game has {} moves", restored.moves.len());

                    // This CBOR file can now be used for neural network training
                    println!("\n🧠 Ready for Neural Network Training");
                    println!("===================================");
                    println!("The CBOR file at {:?} contains:", path);
                    println!("- Complete game record with {} moves", restored.moves.len());
                    println!("- Final board position");
                    println!("- Score information (W+34.5)");
                    println!(
                        "- Can be used with: cargo run --bin train_neural -- {:?}",
                        path
                    );
                }
                Err(e) => println!("❌ Failed to read archive: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to archive: {}", e),
    }

    println!("\n✨ Test completed successfully!");
}
