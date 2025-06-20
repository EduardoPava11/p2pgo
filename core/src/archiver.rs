// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game archive helper functions for the core crate

use chrono::Utc;
use crate::GameState;
use std::path::PathBuf;
use anyhow::Result;
use std::io::Write;

/// Archives a finished game to the local filesystem
///
/// Writes a CBOR file to ~/Library/Application Support/p2pgo/finished/ on macOS
/// or ./finished_games/ on other platforms
///
/// The filename format is: YYYY-MM-DD_vs_<opponent>.cbor
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
    
    // Format the filename
    let sanitized_opponent = opponent.replace(
        |c: char| !c.is_alphanumeric() && c != '-' && c != '_',
        "_"
    );
    let filename = format!("{}_vs_{}.cbor", date, sanitized_opponent);
    let file_path = archive_dir.join(&filename);
    
    // Create a temporary file for atomic write
    let tmp_path = archive_dir.join(format!(".tmp_{}", filename));
    {
        let mut file = std::fs::File::create(&tmp_path)?;
        let cbor_data = serde_cbor::to_vec(&game)?;
        file.write_all(&cbor_data)?;
        file.flush()?;
    }
    
    // Rename for atomic replacement
    std::fs::rename(&tmp_path, &file_path)?;
    
    tracing::info!(
        "Game archived to {:?}",
        file_path
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
                println!("Game archived to {:?}", path);
            },
            Err(e) => {
                eprintln!("Failed to archive game: {}", e);
            }
        }
    });
}
