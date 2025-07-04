// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game state persistence and snapshot management

use anyhow::Result;
use super::GameChannel;

/// Set the directory where game snapshots will be saved
pub async fn set_snapshot_directory(channel: &GameChannel, dir_path: std::path::PathBuf) -> Result<()> {
    let mut snapshot_dir = channel.snapshot_dir.write().await;
    
    // Ensure the directory exists
    tokio::fs::create_dir_all(&dir_path).await?;
    
    // Set the snapshot directory
    *snapshot_dir = Some(dir_path.clone());
    
    tracing::info!(
        game_id = %channel.game_id,
        path = ?dir_path,
        "Snapshot directory set"
    );
    
    Ok(())
}

/// Check if we need to write a snapshot based on move count or time elapsed
pub async fn check_snapshot_needed(channel: &GameChannel) -> bool {
    const MOVES_THRESHOLD: u32 = 10;
    const TIME_THRESHOLD_SECS: u64 = 30;
    
    let moves_count = {
        let moves = channel.moves_since_snapshot.read().await;
        *moves
    };
    
    let elapsed = {
        let last_time = channel.last_snapshot_time.read().await;
        last_time.elapsed()
    };
    
    // Write snapshot if we've made MOVES_THRESHOLD moves or TIME_THRESHOLD_SECS seconds have passed
    moves_count >= MOVES_THRESHOLD || elapsed.as_secs() >= TIME_THRESHOLD_SECS
}

/// Write the current game state as a snapshot
pub async fn write_snapshot(channel: &GameChannel) -> Result<()> {
    // Check if snapshot directory is configured
    let snapshot_dir = {
        let dir = channel.snapshot_dir.read().await;
        match dir.clone() {
            Some(path) => path,
            None => {
                tracing::debug!("No snapshot directory configured, skipping snapshot");
                return Ok(());
            }
        }
    };
    
    // Get the current game state
    let state = match *channel.latest_state.read().await {
        Some(ref state) => state.clone(),
        None => {
            tracing::warn!("No game state available for snapshot");
            return Ok(());
        }
    };
    
    // Create a snapshot filename with game ID
    let temp_filename = format!("{}.snapshot.tmp", channel.game_id);
    let final_filename = format!("{}.snapshot", channel.game_id);
    let temp_path = snapshot_dir.join(temp_filename);
    let final_path = snapshot_dir.join(final_filename);
    
    // Serialize the game state to CBOR
    let cbor_data = p2pgo_core::cbor::serialize_game_state(&state);
    
    // Write the snapshot to a temporary file
    tokio::fs::write(&temp_path, cbor_data).await?;
    
    // Atomically rename the temporary file to the final filename
    // Fall back to copy+delete if rename fails
    match tokio::fs::rename(&temp_path, &final_path).await {
        Ok(_) => {
            tracing::debug!("Renamed snapshot file from {:?} to {:?}", temp_path, final_path);
        }
        Err(e) => {
            tracing::warn!("Failed to rename snapshot file: {}. Falling back to copy+delete", e);
            
            // Copy the file
            tokio::fs::copy(&temp_path, &final_path).await?;
            
            // Verify the copy was successful
            let src_metadata = tokio::fs::metadata(&temp_path).await?;
            let dst_metadata = tokio::fs::metadata(&final_path).await?;
            
            if dst_metadata.len() == src_metadata.len() {
                // Delete the original file only if copy was successful
                if let Err(e) = tokio::fs::remove_file(&temp_path).await {
                    tracing::warn!("Failed to delete temporary snapshot file: {}", e);
                }
            } else {
                return Err(anyhow::anyhow!("Snapshot file copy verification failed: size mismatch"));
            }
        }
    }
    
    // Reset snapshot tracking
    {
        let mut moves = channel.moves_since_snapshot.write().await;
        *moves = 0;
    }
    {
        let mut last_time = channel.last_snapshot_time.write().await;
        *last_time = std::time::Instant::now();
    }
    
    tracing::info!(
        game_id = %channel.game_id,
        path = ?final_path,
        moves = state.moves.len(),
        "Game snapshot written successfully"
    );
    
    Ok(())
}

/// Load a game state from a snapshot file
pub async fn load_snapshot(game_id: &str, snapshot_dir: &std::path::Path) -> Result<Option<p2pgo_core::GameState>> {
    let snapshot_filename = format!("{}.snapshot", game_id);
    let snapshot_path = snapshot_dir.join(snapshot_filename);
    
    // Check if snapshot file exists
    if !tokio::fs::try_exists(&snapshot_path).await? {
        tracing::debug!("No snapshot file found for game {}", game_id);
        return Ok(None);
    }
    
    // Read the snapshot file
    let cbor_data = tokio::fs::read(&snapshot_path).await?;
    
    // Deserialize the game state from CBOR
    match p2pgo_core::cbor::deserialize_game_state(&cbor_data) {
        Some(state) => {
            tracing::info!(
                game_id = %game_id,
                path = ?snapshot_path,
                moves = state.moves.len(),
                "Game snapshot loaded successfully"
            );
            Ok(Some(state))
        }
        None => {
            tracing::error!("Failed to deserialize snapshot for {}", game_id);
            Err(anyhow::anyhow!("Failed to deserialize snapshot"))
        }
    }
}

/// Delete a snapshot file for a game
pub async fn delete_snapshot(game_id: &str, snapshot_dir: &std::path::Path) -> Result<()> {
    let snapshot_filename = format!("{}.snapshot", game_id);
    let snapshot_path = snapshot_dir.join(snapshot_filename);
    
    // Check if snapshot file exists
    if tokio::fs::try_exists(&snapshot_path).await? {
        tokio::fs::remove_file(&snapshot_path).await?;
        tracing::info!(
            game_id = %game_id,
            path = ?snapshot_path,
            "Game snapshot deleted"
        );
    } else {
        tracing::debug!("No snapshot file to delete for game {}", game_id);
    }
    
    Ok(())
}

/// List all available snapshot files in a directory
pub async fn list_snapshots(snapshot_dir: &std::path::Path) -> Result<Vec<String>> {
    let mut game_ids = Vec::new();
    
    if !tokio::fs::try_exists(snapshot_dir).await? {
        return Ok(game_ids);
    }
    
    let mut entries = tokio::fs::read_dir(snapshot_dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".snapshot") {
                    // Extract game ID from filename
                    let game_id = file_name.trim_end_matches(".snapshot");
                    game_ids.push(game_id.to_string());
                }
            }
        }
    }
    
    tracing::debug!("Found {} snapshot files in {:?}", game_ids.len(), snapshot_dir);
    Ok(game_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::GameState;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_snapshot_roundtrip() -> Result<()> {
        // Create a temporary directory
        let temp_dir = TempDir::new()?;
        let snapshot_dir = temp_dir.path();
        
        // Create a test game state
        let mut game_state = GameState::new(9);
        game_state.moves.push(p2pgo_core::Move::Place(p2pgo_core::Coord::new(4, 4)));
        
        // Create a test game channel
        let game_id = "test-snapshot-game";
        let channel = super::super::GameChannel::new(game_id.to_string(), game_state.clone());
        
        // Set snapshot directory
        set_snapshot_directory(&channel, snapshot_dir.to_path_buf()).await?;
        
        // Write snapshot
        write_snapshot(&channel).await?;
        
        // Load snapshot
        let loaded_state = load_snapshot(game_id, snapshot_dir).await?;
        
        // Verify the loaded state matches the original
        assert!(loaded_state.is_some());
        let loaded_state = loaded_state.unwrap();
        assert_eq!(loaded_state.board_size, game_state.board_size);
        assert_eq!(loaded_state.moves.len(), game_state.moves.len());
        
        // Test listing snapshots
        let game_ids = list_snapshots(snapshot_dir).await?;
        assert!(game_ids.contains(&game_id.to_string()));
        
        // Test deleting snapshot
        delete_snapshot(game_id, snapshot_dir).await?;
        let game_ids_after_delete = list_snapshots(snapshot_dir).await?;
        assert!(!game_ids_after_delete.contains(&game_id.to_string()));
        
        Ok(())
    }
}
