//! SGF to CBOR converter for neural network training
//!
//! This module converts SGF game files into CBOR format optimized for neural network training.
//! It replays games move by move, extracting board positions, moves played, and game outcomes.

use crate::cbor_format::{
    create_feature_planes, BatchMetadata, CBORTrainingBatch, ExampleContext, PolicyTarget,
    TrainingExample, TrainingSource,
};
use anyhow::{anyhow, Result};
use p2pgo_core::{Color, GameState, Move};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// SGF to CBOR converter
pub struct SgfToCborConverter {
    /// Board size for the games
    board_size: u8,
    /// Whether to include opening positions
    include_opening: bool,
    /// Whether to include endgame positions
    include_endgame: bool,
    /// Minimum move number to start collecting positions
    min_move_number: usize,
}

impl SgfToCborConverter {
    /// Create a new converter with default settings
    pub fn new(board_size: u8) -> Self {
        Self {
            board_size,
            include_opening: true,
            include_endgame: true,
            min_move_number: 0,
        }
    }

    /// Set whether to include opening positions
    pub fn with_opening(mut self, include: bool) -> Self {
        self.include_opening = include;
        self
    }

    /// Set whether to include endgame positions
    pub fn with_endgame(mut self, include: bool) -> Self {
        self.include_endgame = include;
        self
    }

    /// Set minimum move number to start collecting positions
    pub fn with_min_move(mut self, min_move: usize) -> Self {
        self.min_move_number = min_move;
        self
    }

    /// Convert a single SGF file to CBOR format
    pub fn convert_file(&self, sgf_path: &Path, output_path: &Path) -> Result<()> {
        // Read SGF file
        let sgf_content = fs::read_to_string(sgf_path)?;

        // Parse SGF using core parser
        use p2pgo_core::sgf::SgfProcessor;
        let mut sgf_processor = SgfProcessor::new(GameState::new(self.board_size));
        let final_state = sgf_processor.parse(&sgf_content)?;

        // Extract game metadata
        let source = self.extract_source_info(&sgf_content, sgf_path)?;

        // Replay the game and collect training examples
        let examples = self.replay_and_collect(&final_state, &source)?;

        // Create batch
        let batch = self.create_batch(source, examples)?;

        // Serialize to CBOR and write
        let cbor_data = serde_cbor::to_vec(&batch)?;
        fs::write(output_path, cbor_data)?;

        Ok(())
    }

    /// Convert multiple SGF files to a single CBOR batch
    pub fn convert_batch(&self, sgf_paths: &[&Path], output_path: &Path) -> Result<()> {
        let mut all_examples = Vec::new();
        let mut sources = Vec::new();

        for sgf_path in sgf_paths {
            match self.process_single_sgf(sgf_path) {
                Ok((source, mut examples)) => {
                    sources.push(source);
                    all_examples.append(&mut examples);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to process {}: {}", sgf_path.display(), e);
                }
            }
        }

        if all_examples.is_empty() {
            return Err(anyhow!(
                "No valid training examples extracted from SGF files"
            ));
        }

        // Use first source as primary, but note this is a batch
        let mut primary_source = sources
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No valid sources"))?;
        primary_source.game_id = format!("batch_{}_games", sgf_paths.len());

        // Create and save batch
        let batch = self.create_batch(primary_source, all_examples)?;
        let cbor_data = serde_cbor::to_vec(&batch)?;
        fs::write(output_path, cbor_data)?;

        Ok(())
    }

    /// Process a single SGF file and return source info and examples
    fn process_single_sgf(
        &self,
        sgf_path: &Path,
    ) -> Result<(TrainingSource, Vec<TrainingExample>)> {
        let sgf_content = fs::read_to_string(sgf_path)?;

        use p2pgo_core::sgf::SgfProcessor;
        let mut sgf_processor = SgfProcessor::new(GameState::new(self.board_size));
        let final_state = sgf_processor.parse(&sgf_content)?;

        let source = self.extract_source_info(&sgf_content, sgf_path)?;
        let examples = self.replay_and_collect(&final_state, &source)?;

        Ok((source, examples))
    }

    /// Extract source information from SGF content
    fn extract_source_info(&self, sgf_content: &str, sgf_path: &Path) -> Result<TrainingSource> {
        // Simple extraction - in a real implementation, we'd parse SGF properties properly
        let game_id = sgf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Extract player names and ranks (simplified)
        let black_player = self
            .extract_property(sgf_content, "PB")
            .unwrap_or_else(|| "Unknown".to_string());
        let white_player = self
            .extract_property(sgf_content, "PW")
            .unwrap_or_else(|| "Unknown".to_string());
        let black_rank = self
            .extract_property(sgf_content, "BR")
            .unwrap_or_else(|| "?".to_string());
        let white_rank = self
            .extract_property(sgf_content, "WR")
            .unwrap_or_else(|| "?".to_string());
        let result = self
            .extract_property(sgf_content, "RE")
            .unwrap_or_else(|| "?".to_string());

        Ok(TrainingSource {
            game_id,
            black_player,
            white_player,
            black_rank,
            white_rank,
            result,
        })
    }

    /// Simple property extraction from SGF
    fn extract_property(&self, sgf_content: &str, property: &str) -> Option<String> {
        let pattern = format!("{}[", property);
        if let Some(start) = sgf_content.find(&pattern) {
            let content_start = start + pattern.len();
            if let Some(end) = sgf_content[content_start..].find(']') {
                return Some(sgf_content[content_start..content_start + end].to_string());
            }
        }
        None
    }

    /// Replay game and collect training examples
    fn replay_and_collect(
        &self,
        final_state: &GameState,
        source: &TrainingSource,
    ) -> Result<Vec<TrainingExample>> {
        let mut examples = Vec::new();
        let mut current_state = GameState::new(self.board_size);

        // Determine game outcome for value targets
        let (black_win_value, white_win_value) = if source.result.contains("B+") {
            (1.0, -1.0)
        } else if source.result.contains("W+") {
            (-1.0, 1.0)
        } else {
            (0.0, 0.0) // Draw or unknown
        };

        // Replay moves
        for (move_number, mv) in final_state.moves.iter().enumerate() {
            // Skip early moves if requested
            if move_number < self.min_move_number {
                current_state.apply_move(mv.clone())?;
                continue;
            }

            // Determine position type
            let total_moves = final_state.moves.len();
            let is_opening = move_number < 30;
            let is_endgame = move_number > total_moves - 40;

            // Skip if filtering is enabled
            if !self.include_opening && is_opening {
                current_state.apply_move(mv.clone())?;
                continue;
            }
            if !self.include_endgame && is_endgame {
                current_state.apply_move(mv.clone())?;
                continue;
            }

            // Extract features from current position
            // Create a Board struct from the GameState
            let mut board = p2pgo_core::board::Board::new(self.board_size);
            for y in 0..self.board_size {
                for x in 0..self.board_size {
                    let idx = (y as usize) * (self.board_size as usize) + (x as usize);
                    if let Some(color) = current_state.board[idx] {
                        board.place(p2pgo_core::Coord::new(x, y), color);
                    }
                }
            }

            // For ko point, we need to detect it from game state
            // This is a simplified version - real ko detection would need previous board state
            let ko_point = None; // TODO: Implement proper ko detection

            let features = create_feature_planes(&board, current_state.current_player, ko_point);

            // Create policy target from the move that was played
            let policy_target = match mv {
                Move::Place { x, y, .. } => PolicyTarget::from_move(*x, *y, self.board_size),
                Move::Pass => PolicyTarget {
                    moves: vec![], // Empty for pass
                    board_size: self.board_size,
                },
                Move::Resign => PolicyTarget {
                    moves: vec![], // Empty for resign
                    board_size: self.board_size,
                },
            };

            // Determine value target based on who is to move
            let value_target = match current_state.current_player {
                Color::Black => black_win_value,
                Color::White => white_win_value,
            };

            // Create context
            let context = ExampleContext {
                is_ko_related: ko_point.is_some(),
                is_opening,
                is_endgame,
                move_time: None, // Could extract from SGF if available
            };

            // Add example
            examples.push(TrainingExample {
                move_number,
                features,
                policy_target,
                value_target,
                context,
            });

            // Apply move to get next state
            current_state.apply_move(mv.clone())?;
        }

        Ok(examples)
    }

    /// Create a CBOR training batch
    fn create_batch(
        &self,
        source: TrainingSource,
        examples: Vec<TrainingExample>,
    ) -> Result<CBORTrainingBatch> {
        let example_count = examples.len();

        // Generate batch ID
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let batch_id = format!("{}_{}", source.game_id, timestamp);

        // Calculate quality score (simplified - could be based on player ranks)
        let quality_score = self.calculate_quality_score(&source);

        // Calculate data hash
        let data_hash = self.calculate_hash(&examples)?;

        let metadata = BatchMetadata {
            created_at: timestamp as u64,
            example_count,
            quality_score,
            compressed: false, // Not compressed in this implementation
            data_hash,
        };

        Ok(CBORTrainingBatch {
            batch_id,
            source,
            examples,
            metadata,
        })
    }

    /// Calculate quality score based on player ranks
    fn calculate_quality_score(&self, source: &TrainingSource) -> f32 {
        // Simple heuristic based on player ranks
        let rank_to_score = |rank: &str| -> f32 {
            if rank.contains('d') {
                // Dan player
                let dan_level = rank
                    .chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<f32>()
                    .unwrap_or(1.0);
                0.7 + (dan_level * 0.05).min(0.3)
            } else if rank.contains('k') {
                // Kyu player
                let kyu_level = rank
                    .chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<f32>()
                    .unwrap_or(20.0);
                0.3 + ((20.0 - kyu_level) * 0.02).max(0.0)
            } else {
                0.5 // Unknown rank
            }
        };

        (rank_to_score(&source.black_rank) + rank_to_score(&source.white_rank)) / 2.0
    }

    /// Calculate hash of training examples
    fn calculate_hash(&self, examples: &[TrainingExample]) -> Result<String> {
        let mut hasher = Sha256::new();

        // Hash a summary of the data
        for example in examples {
            hasher.update(example.move_number.to_le_bytes());
            hasher.update(example.value_target.to_le_bytes());
            // Add more fields as needed
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Batch process multiple SGF files in a directory
pub async fn batch_process_directory(
    input_dir: &Path,
    output_dir: &Path,
    batch_size: usize,
) -> Result<()> {
    // Ensure output directory exists
    fs::create_dir_all(output_dir)?;

    // Find all SGF files
    let sgf_files: Vec<_> = fs::read_dir(input_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("sgf"))
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect();

    println!("Found {} SGF files to process", sgf_files.len());

    // Process in batches
    let converter = SgfToCborConverter::new(9); // Assuming 9x9 board

    for (batch_idx, chunk) in sgf_files.chunks(batch_size).enumerate() {
        let output_path = output_dir.join(format!("batch_{:04}.cbor", batch_idx));

        let paths: Vec<&Path> = chunk.iter().map(|p| p.as_path()).collect();

        match converter.convert_batch(&paths, &output_path) {
            Ok(_) => println!("Created batch {}: {}", batch_idx, output_path.display()),
            Err(e) => eprintln!("Failed to create batch {}: {}", batch_idx, e),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_score_calculation() {
        let converter = SgfToCborConverter::new(9);

        let source = TrainingSource {
            game_id: "test".to_string(),
            black_player: "Player1".to_string(),
            white_player: "Player2".to_string(),
            black_rank: "5d".to_string(),
            white_rank: "4d".to_string(),
            result: "B+R".to_string(),
        };

        let score = converter.calculate_quality_score(&source);
        assert!(score > 0.9); // Dan players should have high quality score
    }
}
