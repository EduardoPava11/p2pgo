// SPDX-License-Identifier: MIT OR Apache-2.0

//! Archive rotation tests

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Mock archive manager for testing
pub struct ArchiveManager {
    base_path: std::path::PathBuf,
}

impl ArchiveManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }
    
    pub fn rotate(&self, max_files: usize) -> Result<(), Box<dyn std::error::Error>> {
        let games_dir = self.base_path.join("games");
        let archive_dir = self.base_path.join("archive");
        
        // Create archive dir if it doesn't exist
        fs::create_dir_all(&archive_dir)?;
        
        // Count current files
        let entries: Vec<_> = fs::read_dir(&games_dir)?
            .filter_map(|e| e.ok())
            .collect();
        
        if entries.len() > max_files {
            // Move oldest 25% to archive
            let to_move = entries.len() / 4;
            
            for (i, entry) in entries.iter().enumerate() {
                if i < to_move {
                    let file_name = entry.file_name();
                    let src = entry.path();
                    let dst = archive_dir.join(file_name);
                    fs::rename(src, dst)?;
                }
            }
        }
        
        Ok(())
    }
    
    pub fn count_games(&self) -> usize {
        fs::read_dir(self.base_path.join("games"))
            .map(|entries| entries.count())
            .unwrap_or(0)
    }
    
    pub fn count_archived(&self) -> usize {
        fs::read_dir(self.base_path.join("archive"))
            .map(|entries| entries.count())
            .unwrap_or(0)
    }
}

#[test]
fn test_archive_rotation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let games_dir = temp_dir.path().join("games");
    fs::create_dir_all(&games_dir).expect("Failed to create games dir");
    
    // Create 2001 dummy blob files
    for i in 0..2001 {
        let file_path = games_dir.join(format!("game_{:04}.cbor", i));
        fs::write(&file_path, format!("dummy game data {}", i))
            .expect("Failed to write dummy file");
    }
    
    let manager = ArchiveManager::new(temp_dir.path());
    
    // Perform rotation with limit of 2000
    manager.rotate(2000).expect("Rotation failed");
    
    // Check results
    let remaining_games = manager.count_games();
    let archived_games = manager.count_archived();
    
    // Should have moved 25% of 2001 = ~500 files to archive
    assert_eq!(remaining_games + archived_games, 2001);
    assert!(remaining_games <= 2000);
    assert!(archived_games > 0);
    assert_eq!(archived_games, 2001 / 4); // 25% of total
}

#[test] 
fn test_no_rotation_needed() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let games_dir = temp_dir.path().join("games");
    fs::create_dir_all(&games_dir).expect("Failed to create games dir");
    
    // Create only 1000 files (below threshold)
    for i in 0..1000 {
        let file_path = games_dir.join(format!("game_{:04}.cbor", i));
        fs::write(&file_path, "dummy data").expect("Failed to write file");
    }
    
    let manager = ArchiveManager::new(temp_dir.path());
    manager.rotate(2000).expect("Rotation failed");
    
    // No files should be archived
    assert_eq!(manager.count_games(), 1000);
    assert_eq!(manager.count_archived(), 0);
}
