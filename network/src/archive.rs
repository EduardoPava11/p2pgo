// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game archiving functionality for training data collection

use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::fs;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use p2pgo_core::GameState;
use crate::GameId;

/// Archive metadata for a completed game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameArchive {
    pub game_id: GameId,
    pub final_state: GameState,
    pub move_count: u32,
    pub archived_at: u64, // unix timestamp
    pub winner: Option<p2pgo_core::Color>,
    pub score_diff: Option<i16>,
}

/// Archive manager with rotation after 2000+ games
pub struct ArchiveManager {
    archives: Arc<RwLock<HashMap<GameId, GameArchive>>>,
    max_archives: usize,
    archive_dir: PathBuf,
}

impl ArchiveManager {
    /// Create a new archive manager
    pub fn new() -> Result<Self> {
        let archive_dir = Self::get_archive_directory()?;
        
        Ok(Self {
            archives: Arc::new(RwLock::new(HashMap::new())),
            max_archives: 2000,
            archive_dir,
        })
    }
    
    /// Get the macOS Application Support directory for finished games
    fn get_archive_directory() -> Result<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
            let mut path = PathBuf::from(home);
            path.push("Library");
            path.push("Application Support");
            path.push("p2pgo");
            path.push("finished");
            Ok(path)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for other platforms
            let mut path = PathBuf::from(".");
            path.push("finished_games");
            Ok(path)
        }
    }
    
    /// Archive a completed game
    pub async fn archive_game(&self, game_id: GameId, final_state: GameState, winner: Option<p2pgo_core::Color>, score_diff: Option<i16>) -> Result<()> {
        let _span = tracing::info_span!("network.archive", "ArchiveManager::archive_game").entered();
        
        let move_count = final_state.moves.len() as u32;
        let archive = GameArchive {
            game_id: game_id.clone(),
            final_state,
            move_count,
            archived_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            winner,
            score_diff,
        };
        
        // Ensure archive directory exists
        fs::create_dir_all(&self.archive_dir).await?;
        
        // Save to filesystem as CBOR
        let filename = format!("{}.cbor", game_id);
        let file_path = self.archive_dir.join(filename);
        let cbor_data = serde_cbor::to_vec(&archive)?;
        fs::write(&file_path, cbor_data).await?;
        
        let mut archives = self.archives.write().await;
        
        // Check if rotation is needed
        if archives.len() >= self.max_archives {
            self.rotate_archives(&mut archives).await?;
        }
        
        archives.insert(game_id.clone(), archive);
        
        tracing::info!(
            game_id = %game_id,
            move_count = move_count,
            total_archives = archives.len(),
            file_path = ?file_path,
            "Game archived to filesystem"
        );
        
        Ok(())
    }
    
    /// Rotate archives when limit is reached
    async fn rotate_archives(&self, archives: &mut HashMap<GameId, GameArchive>) -> Result<()> {
        let _span = tracing::info_span!("network.archive", "ArchiveManager::rotate_archives").entered();
        
        // Sort by archived_at timestamp
        let mut archive_list: Vec<(GameId, GameArchive)> = archives.iter()
            .map(|(game_id, archive)| (game_id.clone(), archive.clone()))
            .collect();
        archive_list.sort_by(|a, b| a.1.archived_at.cmp(&b.1.archived_at));
        
        // Remove oldest 25% of archives
        let remove_count = archives.len() / 4;
        let mut removed_count = 0;
        
        // Collect game IDs to remove
        let to_remove: Vec<GameId> = archive_list.iter()
            .take(remove_count)
            .map(|(game_id, _)| game_id.clone())
            .collect();
        
        // Store archives that will be removed to iroh collection if iroh is enabled
        #[cfg(feature = "iroh")]
        {
            // Create a slice of references to the archives that will be removed
            let to_store: Vec<(&GameId, &GameArchive)> = archive_list[..remove_count]
                .iter()
                .map(|(game_id, archive)| (game_id, archive))
                .collect();
            
            // Store the archives to iroh collection
            self.store_to_iroh_collection(&to_store).await?;
        }
        
        // Remove the collected game IDs
        for game_id in to_remove {
            archives.remove(&game_id);
            removed_count += 1;
        }
        
        tracing::info!(
            removed_count = removed_count,
            remaining_count = archives.len(),
            "Archive rotation completed"
        );
        
        Ok(())
    }
    
    #[cfg(feature = "iroh")]
    async fn store_to_iroh_collection(&self, archived_games: &[(&GameId, &GameArchive)]) -> Result<()> {
        let _span = tracing::info_span!("network.archive", "ArchiveManager::store_to_iroh_collection").entered();
        
        // TODO: Implement iroh_docs::Collection storage
        // This would serialize the game archives and store them in a distributed collection
        
        tracing::info!(
            archived_count = archived_games.len(),
            "Archived games stored to iroh collection"
        );
        
        Ok(())
    }
    
    /// Get archive statistics
    pub async fn get_stats(&self) -> (usize, usize) {
        let archives = self.archives.read().await;
        (archives.len(), self.max_archives)
    }
    
    /// Get archived game by ID
    pub async fn get_archive(&self, game_id: &GameId) -> Option<GameArchive> {
        let archives = self.archives.read().await;
        archives.get(game_id).cloned()
    }
    
    /// List all archived games
    pub async fn list_archives(&self) -> Vec<GameArchive> {
        let archives = self.archives.read().await;
        archives.values().cloned().collect()
    }
}

impl Default for ArchiveManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ArchiveManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::{GameState, Color};
    
    #[tokio::test]
    async fn test_archive_game() {
        let manager = ArchiveManager::new().unwrap();
        let game_id = "test-game".to_string();
        let state = GameState::new(9);
        
        manager.archive_game(game_id.clone(), state, Some(Color::Black), Some(5)).await.unwrap();
        
        let archive = manager.get_archive(&game_id).await.unwrap();
        assert_eq!(archive.game_id, game_id);
        assert_eq!(archive.winner, Some(Color::Black));
        assert_eq!(archive.score_diff, Some(5));
    }
    
    #[tokio::test]
    async fn test_archive_rotation() {
        let mut manager = ArchiveManager::new().unwrap();
        manager.max_archives = 10; // Small limit for testing
        
        // Add more than max_archives games
        for i in 0..15 {
            let game_id = format!("game-{}", i);
            let state = GameState::new(9);
            manager.archive_game(game_id, state, None, None).await.unwrap();
            
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        
        let (count, _) = manager.get_stats().await;
        // Should have rotated and kept around 75% of max_archives
        assert!(count <= 10);
        assert!(count >= 7);
    }
}
