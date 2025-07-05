use crate::test_helpers::*;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

use p2pgo_sgf::parser::parse_sgf;
use p2pgo_core::{Board, Color, Move, Point};

#[tokio::test]
async fn test_sgf_to_rna_conversion() -> Result<()> {
    // Load the actual SGF file
    let sgf_path = "/Users/daniel/Downloads/76794817-078-worki-ve..sgf";
    let sgf_content = tokio::fs::read_to_string(sgf_path).await
        .expect("SGF file should exist at the specified path");

    let mut relay = TestRelay::new(4101).await?;
    relay.subscribe_rna().await?;

    // Parse SGF to verify it's valid
    let game = parse_sgf(&sgf_content)?;
    let total_moves = game.main_variation().count();
    info!("SGF file contains {} moves", total_moves);

    // Create RNA from different move ranges
    let test_ranges = vec![
        (0, 20),    // Opening
        (20, 50),   // Middle game
        (50, total_moves.min(80)), // End game
    ];

    for (start, end) in test_ranges {
        let rna = relay.create_sgf_rna(sgf_content.clone(), (start, end));

        assert_eq!(rna.source_peer, relay.peer_id().to_string());
        assert!(rna.quality_score > 0.7, "Professional game should have high quality");

        if let RNAType::SGFData { sgf_content: _, move_range, player_ranks } = &rna.rna_type {
            assert_eq!(move_range, &(start, end));
            assert_eq!(player_ranks, &("7k".to_string(), "4k".to_string()));
        } else {
            panic!("Expected SGFData RNA type");
        }

        // Broadcast RNA
        relay.broadcast_rna(rna.clone()).await?;

        // Verify it's in the gossipsub topic
        let topics = relay.get_subscribed_topics().await;
        assert!(topics.contains(&"p2pgo/rna/v1".to_string()));

        info!("Created RNA for moves {}-{}", start, end);
    }

    Ok(())
}

#[tokio::test]
async fn test_sgf_quality_evaluation() -> Result<()> {
    let relay = TestRelay::new(4102).await?;

    // Test with the provided SGF file
    let sgf_path = "/Users/daniel/Downloads/76794817-078-worki-ve..sgf";
    let sgf_content = tokio::fs::read_to_string(sgf_path).await?;

    let rna = relay.create_sgf_rna(sgf_content.clone(), (0, 50));

    // Parse and evaluate quality
    let moves = test_data::parse_sgf_to_game_state(&rna)
        .expect("Should parse SGF to moves");

    let quality_score = test_data::evaluate_game_quality(&moves);

    info!("SGF quality score: {:.2}", quality_score);
    assert!(quality_score > 0.5, "Real game should have reasonable quality");

    // Test game characteristics
    let game = parse_sgf(&sgf_content)?;
    let metadata = game.root_node();

    assert_eq!(metadata.get_property("BR"), Some(&vec!["7k".to_string()]));
    assert_eq!(metadata.get_property("WR"), Some(&vec!["4k".to_string()]));
    assert_eq!(metadata.get_property("RE"), Some(&vec!["W+34.5".to_string()]));
    assert_eq!(metadata.get_property("SZ"), Some(&vec!["9".to_string()]));

    Ok(())
}

#[tokio::test]
async fn test_training_data_from_sgf() -> Result<()> {
    use p2pgo_neural::training::TrainingData;

    let sgf_path = "/Users/daniel/Downloads/76794817-078-worki-ve..sgf";
    let sgf_content = tokio::fs::read_to_string(sgf_path).await?;

    // Parse SGF
    let game = parse_sgf(&sgf_content)?;
    let mut board = Board::new(9);
    let mut training_positions = Vec::new();

    // Extract training positions from the game
    for (move_num, node) in game.main_variation().enumerate() {
        if let Some((color, point)) = node.get_move() {
            // Create training position before the move
            let mut features = vec![0.0; 8 * 9 * 9]; // 8 feature planes

            // Fill feature planes
            for y in 0..9 {
                for x in 0..9 {
                    let idx = y * 9 + x;
                    let board_point = Point::new(x as u8, y as u8);

                    match board.get(board_point) {
                        Some(Color::Black) => features[idx] = 1.0,
                        Some(Color::White) => features[9 * 9 + idx] = 1.0,
                        None => features[2 * 9 * 9 + idx] = 1.0, // Empty
                    }

                    // Liberties (simplified)
                    if let Some(stone_color) = board.get(board_point) {
                        let liberties = board.get_liberties(board_point).len();
                        let liberty_plane = if stone_color == Color::Black { 3 } else { 4 };
                        features[liberty_plane * 9 * 9 + idx] = liberties.min(8) as f32 / 8.0;
                    }
                }
            }

            // Turn to play
            let turn_plane = if color == Color::Black { 5 } else { 6 };
            for i in 0..81 {
                features[turn_plane * 9 * 9 + i] = 1.0;
            }

            // Move number feature
            for i in 0..81 {
                features[7 * 9 * 9 + i] = (move_num as f32) / 100.0;
            }

            // Policy target (where the move was played)
            let mut policy_target = vec![0.0; 9 * 9];
            policy_target[point.y as usize * 9 + point.x as usize] = 1.0;

            // Value target (simplified - based on final result)
            let value_target = if node.get_property("RE")
                .and_then(|v| v.first())
                .map(|s| s.starts_with("W+"))
                .unwrap_or(false) {
                if color == Color::White { 1.0 } else { -1.0 }
            } else {
                if color == Color::Black { 1.0 } else { -1.0 }
            };

            training_positions.push(TrainingData {
                features,
                policy_target,
                value_target,
            });

            // Make the move
            board.place_stone(point, color)?;
        }
    }

    info!("Extracted {} training positions from SGF", training_positions.len());
    assert!(training_positions.len() > 50, "Should have many training positions");

    // Verify training data quality
    for (i, data) in training_positions.iter().take(5).enumerate() {
        assert_eq!(data.features.len(), 8 * 9 * 9);
        assert_eq!(data.policy_target.len(), 9 * 9);
        assert!(data.policy_target.iter().sum::<f32>().abs() - 1.0 < 0.001);
        assert!(data.value_target.abs() <= 1.0);

        info!("Position {}: value_target = {}", i, data.value_target);
    }

    Ok(())
}

#[tokio::test]
async fn test_sgf_upload_tool_simulation() -> Result<()> {
    // Simulate the SGF upload tool workflow
    let sgf_path = PathBuf::from("/Users/daniel/Downloads/76794817-078-worki-ve..sgf");
    assert!(sgf_path.exists(), "SGF file should exist");

    let sgf_content = tokio::fs::read_to_string(&sgf_path).await?;

    // Parse game
    let game = parse_sgf(&sgf_content)?;
    let total_moves = game.main_variation().count();

    // Simulate move range selection
    let move_ranges = vec![
        (0, total_moves),           // Full game
        (0, 20),                    // Opening only
        (10, 40),                   // Custom range
        (total_moves - 20, total_moves), // Endgame
    ];

    for (start, end) in move_ranges {
        info!("Testing range {}-{} of {}", start, end, total_moves);

        // Extract moves for the range
        let moves: Vec<_> = game.main_variation()
            .skip(start)
            .take(end - start)
            .filter_map(|node| node.get_move())
            .collect();

        assert_eq!(moves.len(), end - start, "Should extract correct number of moves");

        // Verify move data
        for (i, (color, point)) in moves.iter().take(5).enumerate() {
            info!("Move {}: {} at ({}, {})",
                start + i,
                if *color == Color::Black { "Black" } else { "White" },
                point.x, point.y
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_sgf_metadata_extraction() -> Result<()> {
    let sgf_path = "/Users/daniel/Downloads/76794817-078-worki-ve..sgf";
    let sgf_content = tokio::fs::read_to_string(sgf_path).await?;

    let game = parse_sgf(&sgf_content)?;
    let root = game.root_node();

    // Extract metadata
    let metadata = SGFMetadata {
        black_player: root.get_property("PB")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        white_player: root.get_property("PW")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        black_rank: root.get_property("BR")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        white_rank: root.get_property("WR")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        result: root.get_property("RE")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        board_size: root.get_property("SZ")
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(19),
        komi: root.get_property("KM")
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(7.5),
        date: root.get_property("DT")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
    };

    info!("SGF Metadata: {:?}", metadata);

    assert_eq!(metadata.black_player, "ve.");
    assert_eq!(metadata.white_player, "worki");
    assert_eq!(metadata.black_rank, "7k");
    assert_eq!(metadata.white_rank, "4k");
    assert_eq!(metadata.result, "W+34.5");
    assert_eq!(metadata.board_size, 9);
    assert_eq!(metadata.komi, 7.5);

    Ok(())
}

#[derive(Debug)]
struct SGFMetadata {
    black_player: String,
    white_player: String,
    black_rank: String,
    white_rank: String,
    result: String,
    board_size: usize,
    komi: f32,
    date: String,
}