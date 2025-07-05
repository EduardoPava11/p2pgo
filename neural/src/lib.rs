pub mod config;
pub mod federated;
pub mod neural_trainer;
pub mod relay_net;
pub mod simple_net;
pub mod storage;
pub mod training;

// CBOR training format (requires candle feature)
#[cfg(feature = "candle")]
pub mod cbor_training;

// CBOR format without candle dependency
pub mod cbor_format;

use p2pgo_core::GameState;
use serde::{Deserialize, Serialize};

// Re-export key types
pub use config::{ConfigWizard, NeuralConfig, NeuralWeights};
pub use neural_trainer::{NeuralTrainer, TrainingResult, TrainingStats};
pub use simple_net::{SimpleNeuralNet, TrainingData};

/// Neural network move prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePrediction {
    pub coord: p2pgo_core::Coord,
    pub probability: f32,
}

/// Board evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardEvaluation {
    /// Win probability for current player (-1 to 1)
    pub win_probability: f32,
    /// Confidence in evaluation (0 to 1)
    pub confidence: f32,
}

/// Dual neural network system like AlphaGo
pub struct DualNeuralNet {
    /// Simple implementation for now
    simple_net: SimpleNeuralNet,
}

impl DualNeuralNet {
    pub fn new() -> Self {
        Self {
            simple_net: SimpleNeuralNet::new(),
        }
    }

    /// Get move predictions from policy network
    pub fn predict_moves(&self, game_state: &GameState) -> Vec<MovePrediction> {
        self.simple_net.predict_moves(game_state)
    }

    /// Evaluate board position with value network
    pub fn evaluate_position(&self, game_state: &GameState) -> BoardEvaluation {
        self.simple_net.evaluate_position(game_state)
    }

    /// Get heat map data for UI visualization
    pub fn get_heat_map(&self, game_state: &GameState) -> [[f32; 19]; 19] {
        let predictions = self.predict_moves(game_state);
        let mut heat_map = [[0.0; 19]; 19];

        for pred in predictions {
            if pred.coord.x < 19 && pred.coord.y < 19 {
                heat_map[pred.coord.y as usize][pred.coord.x as usize] = pred.probability;
            }
        }

        heat_map
    }
}

impl Default for DualNeuralNet {
    fn default() -> Self {
        Self::new()
    }
}
