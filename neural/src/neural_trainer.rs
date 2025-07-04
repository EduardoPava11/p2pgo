//! Training system for neural networks from SGF files

use crate::{SimpleNeuralNet, TrainingData, config::NeuralConfig};
use p2pgo_core::{GameState, Move, Color, SGFParser};
use std::path::Path;
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// Training system for neural networks
pub struct NeuralTrainer {
    pub network: SimpleNeuralNet,
    pub config: NeuralConfig,
    pub training_data: Vec<TrainingData>,
    pub games_trained: usize,
}

impl NeuralTrainer {
    /// Create a new trainer with configuration
    pub fn new(config: NeuralConfig) -> Self {
        Self {
            network: SimpleNeuralNet::new_with_config(&config),
            config,
            training_data: Vec::new(),
            games_trained: 0,
        }
    }
    
    /// Load and train from a single SGF file
    pub async fn train_from_sgf(&mut self, sgf_path: &Path) -> Result<TrainingResult> {
        // Parse SGF file
        let parsed_game = SGFParser::parse_file(sgf_path)?;
        
        // Extract training data
        let mut training = TrainingData::new();
        let mut current_state = GameState::new(parsed_game.final_state.board_size);
        
        // Process each move
        for mv in &parsed_game.moves {
            // Add position before move
            training.add_position(current_state.clone(), mv.clone());
            
            // Apply move to get next state
            current_state.apply_move(mv.clone())?;
        }
        
        // Set winner if known
        let result = &parsed_game.metadata.result;
        if result.contains("B+") {
            training.set_winner(Color::Black);
        } else if result.contains("W+") {
            training.set_winner(Color::White);
        }
        
        // Train the network
        let moves_learned = training.moves.len();
        self.learn_from_game(&training)?;
        
        // Store training data
        self.training_data.push(training);
        self.games_trained += 1;
        
        Ok(TrainingResult {
            game_id: parsed_game.id,
            moves_learned,
            training_time_ms: 0, // Would measure actual time
            accuracy_before: 0.0,
            accuracy_after: 0.0,
        })
    }
    
    /// Batch train from multiple SGF files
    pub async fn train_from_sgf_batch(&mut self, sgf_paths: &[&Path]) -> Result<BatchTrainingResult> {
        let mut results = Vec::new();
        let mut total_moves = 0;
        
        for path in sgf_paths {
            match self.train_from_sgf(path).await {
                Ok(result) => {
                    total_moves += result.moves_learned;
                    results.push(result);
                }
                Err(e) => {
                    eprintln!("Failed to train from {:?}: {}", path, e);
                }
            }
        }
        
        Ok(BatchTrainingResult {
            games_processed: results.len(),
            total_moves_learned: total_moves,
            individual_results: results,
        })
    }
    
    /// Learn from a single game
    fn learn_from_game(&mut self, training: &TrainingData) -> Result<()> {
        let weights = self.config.to_weights();
        
        // Update pattern weights based on moves
        for (i, state) in training.states().iter().enumerate() {
            if let Some(mv) = training.moves.get(i) {
                // Update network patterns based on configuration
                self.network.update_patterns(state, mv, &weights);
            }
        }
        
        Ok(())
    }
    
    /// Get training statistics
    pub fn get_stats(&self) -> TrainingStats {
        TrainingStats {
            games_trained: self.games_trained,
            total_positions: self.training_data.iter()
                .map(|td| td.moves.len())
                .sum(),
            config: self.config.clone(),
        }
    }
    
    /// Save trained network
    pub fn save(&self, path: &Path) -> Result<()> {
        let data = SavedNetwork {
            network: self.network.clone(),
            config: self.config.clone(),
            games_trained: self.games_trained,
        };
        
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Load trained network
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let data: SavedNetwork = serde_json::from_str(&json)?;
        
        Ok(Self {
            network: data.network,
            config: data.config,
            training_data: Vec::new(),
            games_trained: data.games_trained,
        })
    }
}

/// Result of training from a single game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    pub game_id: String,
    pub moves_learned: usize,
    pub training_time_ms: u64,
    pub accuracy_before: f32,
    pub accuracy_after: f32,
}

/// Result of batch training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTrainingResult {
    pub games_processed: usize,
    pub total_moves_learned: usize,
    pub individual_results: Vec<TrainingResult>,
}

/// Training statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStats {
    pub games_trained: usize,
    pub total_positions: usize,
    pub config: NeuralConfig,
}

/// Saved network format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedNetwork {
    network: SimpleNeuralNet,
    config: NeuralConfig,
    games_trained: usize,
}

impl SimpleNeuralNet {
    /// Create network with specific configuration
    pub fn new_with_config(config: &NeuralConfig) -> Self {
        let mut net = Self::new();
        
        // Apply configuration to pattern weights
        let weights = config.to_weights();
        
        // Adjust initial patterns based on style
        if config.aggression > 7 {
            net.add_pattern("attack_33", weights.attack_weight * 0.9);
            net.add_pattern("cut_point", weights.attack_weight * 0.8);
        }
        
        if config.territory_focus > 7 {
            net.add_pattern("corner_enclosure", weights.territory_weight * 0.9);
            net.add_pattern("side_extension", weights.territory_weight * 0.8);
        }
        
        net
    }
    
    /// Add a pattern with weight
    pub fn add_pattern(&mut self, name: &str, weight: f32) {
        self.pattern_weights.insert(name.to_string(), weight);
    }
    
    /// Update patterns based on a move
    pub fn update_patterns(&mut self, _state: &GameState, _mv: &Move, weights: &crate::config::NeuralWeights) {
        // Simple learning: increase weight of patterns that led to good moves
        // This is where more sophisticated learning would happen
        
        // For now, just adjust some pattern weights slightly
        for (pattern, weight) in self.pattern_weights.iter_mut() {
            if pattern.contains("corner") {
                *weight *= 1.0 + (weights.learning_rate * 0.1);
            }
        }
    }
}