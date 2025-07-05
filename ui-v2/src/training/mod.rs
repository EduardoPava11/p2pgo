//! Training module for neural network training from SGF files

use anyhow::Result;
use p2pgo_core::training_pipeline::{TrainingConfig, TrainingPipeline};
use p2pgo_neural::training::sgf_to_cbor::SgfToCborConverter;
use p2pgo_neural::training::TrainingStats;
use std::path::PathBuf;
use tracing::{error, info};

/// Train neural network from SGF files
pub async fn train_from_sgf_files(
    sgf_files: Vec<PathBuf>,
    progress_callback: impl Fn(f32) + Send + 'static,
) -> Result<TrainingStats> {
    info!("Starting training with {} SGF files", sgf_files.len());

    // Create temporary directory for CBOR files
    let temp_dir = tempfile::tempdir()?;
    let cbor_dir = temp_dir.path();

    // Convert SGF files to CBOR format
    info!("Converting SGF files to CBOR format...");
    progress_callback(0.1); // 10% - starting conversion

    let converter = SgfToCborConverter::new(9); // Default to 9x9 board

    for (idx, sgf_path) in sgf_files.iter().enumerate() {
        let cbor_path = cbor_dir.join(format!("training_{:03}.cbor", idx));

        match converter.convert_file(sgf_path, &cbor_path) {
            Ok(_) => info!("Converted {} to CBOR", sgf_path.display()),
            Err(e) => {
                error!("Failed to convert {}: {}", sgf_path.display(), e);
                // Continue with other files
            }
        }

        // Update progress
        let conversion_progress = 0.1 + (0.3 * (idx + 1) as f32 / sgf_files.len() as f32);
        progress_callback(conversion_progress);
    }

    info!("Conversion complete. Starting neural network training...");
    progress_callback(0.4); // 40% - starting training

    // Create training pipeline
    let config = TrainingConfig {
        board_size: 9,
        epochs: 10,
        batch_size: 32,
        learning_rate: 0.001,
        use_gpu: false, // Conservative for macOS
        min_games: 1,   // Allow training with few games for testing
    };

    let pipeline = TrainingPipeline::new(config)?;

    // Train from CBOR data
    let results = pipeline.train_from_data(cbor_dir)?;

    progress_callback(0.9); // 90% - training complete

    // Convert results to training stats
    let stats = TrainingStats {
        games_trained: sgf_files.len(),
        total_positions: results.total_examples,
        sword_accuracy: results.sword_accuracy,
        shield_accuracy: results.shield_accuracy,
        training_time_secs: 0, // Would need to track actual time
        final_loss: 0.0,       // Not tracked in current implementation
    };

    info!("Training completed: {}", results.summary());
    progress_callback(1.0); // 100% - done

    Ok(stats)
}

/// Training message for UI updates
#[derive(Debug, Clone)]
pub enum TrainingMessage {
    Progress(f32),
    Completed(TrainingStats),
    Error(String),
}
