//! CBOR training data format without candle dependency
//! This is a simplified version that works with the SGF to CBOR converter

use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;
use std::fs;

/// CBOR Training Data Format for P2P Go Neural Networks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CBORTrainingBatch {
    /// Unique batch ID
    pub batch_id: String,
    /// Source information
    pub source: TrainingSource,
    /// Training examples
    pub examples: Vec<TrainingExample>,
    /// Metadata
    pub metadata: BatchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSource {
    /// SGF file or game ID
    pub game_id: String,
    /// Player information
    pub black_player: String,
    pub white_player: String,
    pub black_rank: String,
    pub white_rank: String,
    /// Game result
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    /// Move number in the game
    pub move_number: usize,
    /// Feature planes (8 planes for P2P Go)
    pub features: FeaturePlanes,
    /// Policy target (move probabilities)
    pub policy_target: PolicyTarget,
    /// Value target (position evaluation)
    pub value_target: f32,
    /// Additional context
    pub context: ExampleContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturePlanes {
    /// 8 feature planes, each board_size x board_size
    /// Stored as flattened vectors for efficiency
    pub planes: Vec<Vec<f32>>,
    pub board_size: u8,
}

impl FeaturePlanes {
    pub fn new(board_size: u8) -> Self {
        let plane_size = (board_size * board_size) as usize;
        Self {
            planes: vec![vec![0.0; plane_size]; 8],
            board_size,
        }
    }
    
    /// Set a value in a specific plane
    pub fn set(&mut self, plane: usize, x: usize, y: usize, value: f32) {
        let idx = y * self.board_size as usize + x;
        self.planes[plane][idx] = value;
    }
    
    /// Get a value from a specific plane
    pub fn get(&self, plane: usize, x: usize, y: usize) -> f32 {
        let idx = y * self.board_size as usize + x;
        self.planes[plane][idx]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTarget {
    /// Sparse representation: (x, y, probability)
    pub moves: Vec<(u8, u8, f32)>,
    pub board_size: u8,
}

impl PolicyTarget {
    /// Convert to dense vector for training
    pub fn to_dense(&self) -> Vec<f32> {
        let size = (self.board_size * self.board_size) as usize;
        let mut dense = vec![0.0; size];
        
        for (x, y, prob) in &self.moves {
            let idx = (*y as usize) * (self.board_size as usize) + (*x as usize);
            dense[idx] = *prob;
        }
        
        dense
    }
    
    /// Create from a single move
    pub fn from_move(x: u8, y: u8, board_size: u8) -> Self {
        Self {
            moves: vec![(x, y, 1.0)],
            board_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleContext {
    /// Is this position part of a Ko sequence?
    pub is_ko_related: bool,
    /// Is this an opening position?
    pub is_opening: bool,
    /// Is this an endgame position?
    pub is_endgame: bool,
    /// Move timing (seconds)
    pub move_time: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// When this batch was created
    pub created_at: u64,
    /// Total examples in batch
    pub example_count: usize,
    /// Average game quality score
    pub quality_score: f32,
    /// Compression used
    pub compressed: bool,
    /// Hash of the data for verification
    pub data_hash: String,
}

/// Feature plane definitions for P2P Go
pub mod feature_planes {
    pub const BLACK_STONES: usize = 0;
    pub const WHITE_STONES: usize = 1;
    pub const EMPTY_POINTS: usize = 2;
    pub const BLACK_LIBERTIES: usize = 3;
    pub const WHITE_LIBERTIES: usize = 4;
    pub const BLACK_TO_PLAY: usize = 5;
    pub const WHITE_TO_PLAY: usize = 6;
    pub const KO_POINTS: usize = 7;
}

/// Create feature planes from board state
pub fn create_feature_planes(
    board: &p2pgo_core::board::Board,
    next_player: p2pgo_core::Color,
    ko_point: Option<p2pgo_core::Coord>,
) -> FeaturePlanes {
    let board_size = board.size();
    let mut planes = FeaturePlanes::new(board_size);
    
    // Fill feature planes
    for y in 0..board_size {
        for x in 0..board_size {
            let coord = p2pgo_core::Coord::new(x, y);
            
            match board.get(coord) {
                Some(p2pgo_core::Color::Black) => {
                    planes.set(feature_planes::BLACK_STONES, x as usize, y as usize, 1.0);
                    
                    // Count liberties
                    let liberties = count_liberties(board, coord);
                    let liberty_value = (liberties as f32 / 4.0).min(1.0);
                    planes.set(feature_planes::BLACK_LIBERTIES, x as usize, y as usize, liberty_value);
                }
                Some(p2pgo_core::Color::White) => {
                    planes.set(feature_planes::WHITE_STONES, x as usize, y as usize, 1.0);
                    
                    // Count liberties
                    let liberties = count_liberties(board, coord);
                    let liberty_value = (liberties as f32 / 4.0).min(1.0);
                    planes.set(feature_planes::WHITE_LIBERTIES, x as usize, y as usize, liberty_value);
                }
                None => {
                    planes.set(feature_planes::EMPTY_POINTS, x as usize, y as usize, 1.0);
                }
            }
        }
    }
    
    // Set next player plane
    let player_plane = match next_player {
        p2pgo_core::Color::Black => feature_planes::BLACK_TO_PLAY,
        p2pgo_core::Color::White => feature_planes::WHITE_TO_PLAY,
    };
    
    for y in 0..board_size {
        for x in 0..board_size {
            planes.set(player_plane, x as usize, y as usize, 1.0);
        }
    }
    
    // Mark Ko point if any
    if let Some(ko) = ko_point {
        planes.set(feature_planes::KO_POINTS, ko.x as usize, ko.y as usize, 1.0);
    }
    
    planes
}

fn count_liberties(board: &p2pgo_core::board::Board, coord: p2pgo_core::Coord) -> usize {
    coord.adjacent_coords().iter()
        .filter(|&&adj| adj.is_valid(board.size()) && board.get(adj).is_none())
        .count()
}

/// Simple CBOR data loader
pub struct CBORDataLoader;

impl CBORDataLoader {
    pub fn new() -> Self {
        Self
    }
    
    /// Load a CBOR training batch from file
    pub fn load_batch(&self, path: &Path) -> Result<CBORTrainingBatch> {
        let data = fs::read(path)?;
        let batch: CBORTrainingBatch = serde_cbor::from_slice(&data)?;
        Ok(batch)
    }
    
    /// Save a CBOR training batch to file
    pub fn save_batch(&self, batch: &CBORTrainingBatch, path: &Path) -> Result<()> {
        let data = serde_cbor::to_vec(batch)?;
        fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature_planes() {
        let planes = FeaturePlanes::new(9);
        assert_eq!(planes.planes.len(), 8);
        assert_eq!(planes.planes[0].len(), 81);
    }
    
    #[test]
    fn test_policy_target() {
        let policy = PolicyTarget::from_move(3, 3, 9);
        let dense = policy.to_dense();
        assert_eq!(dense.len(), 81);
        assert_eq!(dense[3 * 9 + 3], 1.0);
    }
}