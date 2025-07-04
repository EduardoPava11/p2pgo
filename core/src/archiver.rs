// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game archive helper functions for the core crate

use chrono::Utc;
use crate::GameState;
use std::path::PathBuf;
use anyhow::{Result, Context};
use std::io::Write;
use flate2::Compression;
use flate2::write::GzEncoder;

// Threshold for gzip compression (1 MiB)
const COMPRESSION_THRESHOLD: usize = 1024 * 1024;

/// Archives a finished game to the local filesystem
///
/// Writes a CBOR file to ~/Library/Application Support/p2pgo/finished/ on macOS
/// or ./finished_games/ on other platforms. If the CBOR data exceeds 1 MiB,
/// it will be gzip compressed and saved with a .cbor.gz extension.
///
/// The filename format is: YYYY-MM-DD_vs_<opponent>.cbor(.gz)
///
/// # Arguments
/// * `game` - The final GameState to archive
/// * `opponent` - Name of the opponent
///
/// # Returns
/// * `Result<PathBuf>` - Path to the archived file on success
pub fn archive_finished_game(game: &GameState, opponent: &str) -> Result<PathBuf> {
    // Format the date in YYYY-MM-DD format
    let date = Utc::now().format("%Y-%m-%d").to_string();
    
    // Get the appropriate directory for the platform
    let archive_dir = match std::env::consts::OS {
        "macos" => {
            let mut path = PathBuf::from(
                std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?
            );
            path.push("Library");
            path.push("Application Support");
            path.push("p2pgo");
            path.push("finished");
            path
        },
        _ => {
            let mut path = PathBuf::from(".");
            path.push("finished_games");
            path
        }
    };
    
    // Ensure the directory exists
    std::fs::create_dir_all(&archive_dir)?;
    
    // Format the filename base
    let sanitized_opponent = opponent.replace(
        |c: char| !c.is_alphanumeric() && c != '-' && c != '_',
        "_"
    );
    let filename_base = format!("{}_vs_{}", date, sanitized_opponent);
    
    // Serialize the game data to CBOR
    let cbor_data = serde_cbor::to_vec(&game)
        .context("Failed to serialize game to CBOR")?;
    
    // Choose whether to use compression based on data size
    let (final_data, extension) = if cbor_data.len() >= COMPRESSION_THRESHOLD {
        // Use gzip compression for large files
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&cbor_data)
            .context("Failed to compress game data")?;
        let compressed_data = encoder.finish()
            .context("Failed to finish compression")?;
        
        tracing::info!(
            "Compressed game archive from {} bytes to {} bytes ({}% reduction)",
            cbor_data.len(),
            compressed_data.len(),
            (1.0 - (compressed_data.len() as f64 / cbor_data.len() as f64)) * 100.0
        );
        
        (compressed_data, "cbor.gz")
    } else {
        // Use uncompressed CBOR for small files
        (cbor_data, "cbor")
    };
    
    let filename = format!("{}.{}", filename_base, extension);
    let file_path = archive_dir.join(&filename);
    
    // Create a temporary file for atomic write
    let tmp_path = archive_dir.join(format!(".tmp_{}", filename));
    {
        let mut file = std::fs::File::create(&tmp_path)
            .context("Failed to create temporary file")?;
        file.write_all(&final_data)
            .context("Failed to write archive data")?;
        file.flush()
            .context("Failed to flush file buffer")?;
    }
    
    // Rename for atomic replacement
    std::fs::rename(&tmp_path, &file_path)
        .context("Failed to rename temporary file")?;
    
    tracing::info!(
        "Game archived to {:?} ({} bytes)",
        file_path,
        final_data.len()
    );
    
    Ok(file_path)
}

/// Archives a finished game asynchronously
///
/// Same as `archive_finished_game` but runs in a background task
///
/// # Arguments
/// * `game` - The final GameState to archive
/// * `opponent` - Name of the opponent
pub fn archive_finished_game_async(game: GameState, opponent: String) {
    // Clone data for the async task
    std::thread::spawn(move || {
        match archive_finished_game(&game, &opponent) {
            Ok(path) => {
                tracing::info!("Game archived to {:?}", path);
            },
            Err(e) => {
                tracing::error!("Failed to archive game: {}", e);
            }
        }
    });
}

/// Read a game archive file, handling both compressed and uncompressed formats
pub fn read_game_archive(path: &PathBuf) -> Result<GameState> {
    use std::io::Read;
    
    let mut file = std::fs::File::open(path)
        .context("Failed to open game archive file")?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .context("Failed to read archive file")?;
    
    // Check if it's a gzipped file by extension
    if path.to_string_lossy().ends_with(".gz") {
        // Decompress the data
        let mut decoder = flate2::read::GzDecoder::new(&buffer[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .context("Failed to decompress archive data")?;
        
        // Parse the CBOR data
        serde_cbor::from_slice(&decompressed)
            .context("Failed to parse decompressed CBOR data")
    } else {
        // Parse directly as CBOR
        serde_cbor::from_slice(&buffer)
            .context("Failed to parse CBOR data") 
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameState, Move, Coord};
    
    #[test]
    fn test_archive_compression() {
        // Create a large game state
        let mut game = GameState::new(19);
        
        // Add large vector data to make it large
        // Create a lot of moves to make the game state larger
        for i in 0..100 {
            game.moves.push(Move::Place(Coord::new(i % 19, i / 19)));
        }
        
        // Archive the game
        let result = archive_finished_game(&game, "test_opponent");
        assert!(result.is_ok(), "Archive failed: {:?}", result.err());
        
        let path = result.unwrap();
        assert!(path.exists(), "Archive file does not exist");
        
        // Verify file extension - with enough moves, it should be compressed
        let data_size = serde_cbor::to_vec(&game).unwrap().len();
        if data_size >= COMPRESSION_THRESHOLD {
            assert!(path.to_string_lossy().ends_with(".cbor.gz"), 
                    "Large data ({} bytes) should be compressed with .cbor.gz extension", data_size);
        } else {
            assert!(path.to_string_lossy().ends_with(".cbor"), 
                    "Small data ({} bytes) should have .cbor extension", data_size);
        }
        
        // Clean up
        std::fs::remove_file(path).ok();
    }
    
    #[test]
    fn test_read_archived_game() {
        // Create a game state
        let original_game = GameState::new(19);
        
        // Archive the game
        let path = archive_finished_game(&original_game, "test").unwrap();
        
        // Read it back
        let loaded_game = read_game_archive(&path).unwrap();
        
        // Verify it's the same
        assert_eq!(original_game.board_size, loaded_game.board_size);
        assert_eq!(original_game.current_player, loaded_game.current_player);
        assert_eq!(original_game.moves.len(), loaded_game.moves.len());
        
        // Clean up
        std::fs::remove_file(path).ok();
    }
}
