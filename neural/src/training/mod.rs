//! Training modules for neural networks

pub mod sgf_to_cbor;

pub use sgf_to_cbor::{SgfToCborConverter, batch_process_directory};

/// Training statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingStats {
    /// Number of games trained
    pub games_trained: usize,
    /// Total number of positions
    pub total_positions: usize,
    /// Sword policy accuracy
    pub sword_accuracy: f32,
    /// Shield policy accuracy 
    pub shield_accuracy: f32,
    /// Training time in seconds
    pub training_time_secs: u64,
    /// Final loss value
    pub final_loss: f32,
}