//! Tests for SGF to CBOR converter

use p2pgo_neural::training::SgfToCborConverter;
use p2pgo_neural::cbor_format::{CBORDataLoader, feature_planes};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_sgf_to_cbor_conversion() {
    // Create a simple SGF for testing
    let sgf_content = r#"(;FF[4]GM[1]SZ[9]
PB[Black Player]PW[White Player]
BR[5k]WR[4k]
RE[B+2.5]
;B[ee];W[eg];B[ce];W[cg];B[gc];W[gf];B[dd];W[df])"#;
    
    // Create temp directory
    let temp_dir = TempDir::new().unwrap();
    let sgf_path = temp_dir.path().join("test.sgf");
    let cbor_path = temp_dir.path().join("test.cbor");
    
    // Write SGF file
    fs::write(&sgf_path, sgf_content).unwrap();
    
    // Convert to CBOR
    let converter = SgfToCborConverter::new(9);
    converter.convert_file(&sgf_path, &cbor_path).unwrap();
    
    // Verify CBOR file was created
    assert!(cbor_path.exists());
    
    // Load and verify CBOR content
    let loader = CBORDataLoader::new();
    let batch = loader.load_batch(&cbor_path).unwrap();
    
    // Check batch properties
    assert_eq!(batch.source.black_player, "Black Player");
    assert_eq!(batch.source.white_player, "White Player");
    assert_eq!(batch.source.black_rank, "5k");
    assert_eq!(batch.source.white_rank, "4k");
    assert_eq!(batch.source.result, "B+2.5");
    
    // Check examples were generated (8 moves in the game)
    assert_eq!(batch.examples.len(), 8);
    
    // Verify first example
    let first_example = &batch.examples[0];
    assert_eq!(first_example.move_number, 0);
    assert_eq!(first_example.features.board_size, 9);
    assert_eq!(first_example.features.planes.len(), 8);
    
    // Check that black is to play in first position
    let black_to_play_plane = &first_example.features.planes[feature_planes::BLACK_TO_PLAY];
    assert!(black_to_play_plane.iter().all(|&v| v == 1.0));
    
    // Check policy target for first move (e4 = 4,4)
    assert_eq!(first_example.policy_target.moves.len(), 1);
    let (x, y, prob) = first_example.policy_target.moves[0];
    assert_eq!(x, 4);
    assert_eq!(y, 4);
    assert_eq!(prob, 1.0);
    
    // Check value target (black won)
    assert_eq!(first_example.value_target, 1.0);
}

#[test]
fn test_batch_conversion() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple SGF files
    let sgf_contents = vec![
        r#"(;FF[4]GM[1]SZ[9]RE[B+R];B[ee];W[eg])"#,
        r#"(;FF[4]GM[1]SZ[9]RE[W+5.5];B[dd];W[df])"#,
        r#"(;FF[4]GM[1]SZ[9]RE[B+0.5];B[ed];W[ef])"#,
    ];
    
    let mut sgf_paths = Vec::new();
    for (i, content) in sgf_contents.iter().enumerate() {
        let path = temp_dir.path().join(format!("game{}.sgf", i));
        fs::write(&path, content).unwrap();
        sgf_paths.push(path);
    }
    
    // Convert batch
    let cbor_path = temp_dir.path().join("batch.cbor");
    let converter = SgfToCborConverter::new(9);
    
    let path_refs: Vec<&Path> = sgf_paths.iter().map(|p| p.as_path()).collect();
    converter.convert_batch(&path_refs, &cbor_path).unwrap();
    
    // Load and verify
    let loader = CBORDataLoader::new();
    let batch = loader.load_batch(&cbor_path).unwrap();
    
    // Should have examples from all games
    assert!(batch.examples.len() >= 6); // At least 2 moves per game
    assert!(batch.metadata.example_count > 0);
}

#[test]
fn test_filtering_options() {
    let sgf_content = r#"(;FF[4]GM[1]SZ[9]RE[B+R]
;B[ee];W[eg];B[ce];W[cg];B[gc];W[gf];B[dd];W[df]
;B[fc];W[ff];B[cd];W[cf];B[ec];W[ef];B[dc];W[de])"#;
    
    let temp_dir = TempDir::new().unwrap();
    let sgf_path = temp_dir.path().join("test.sgf");
    fs::write(&sgf_path, sgf_content).unwrap();
    
    // Test with minimum move filter
    let converter = SgfToCborConverter::new(9).with_min_move(5);
    let cbor_path = temp_dir.path().join("filtered.cbor");
    converter.convert_file(&sgf_path, &cbor_path).unwrap();
    
    let loader = CBORDataLoader::new();
    let batch = loader.load_batch(&cbor_path).unwrap();
    
    // Should only have examples from move 5 onwards
    assert!(batch.examples.len() < 16); // Less than total moves
    assert_eq!(batch.examples[0].move_number, 5);
}