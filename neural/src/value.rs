use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{linear, ops, Linear, VarBuilder};
use p2pgo_core::{BoardState, Color, GameState};

/// Value network - evaluates board positions
/// Input: 19x19 board state with features
/// Output: Single value representing win probability
pub struct ValueNetwork {
    /// Input features (same as policy network)
    input_size: usize,
    /// Hidden layers
    hidden1: Linear,
    hidden2: Linear,
    hidden3: Linear,
    hidden4: Linear,
    /// Output layer - single value
    output: Linear,
    device: Device,
}

impl ValueNetwork {
    pub fn new() -> Self {
        let device = Device::Cpu;
        let dtype = DType::F32;
        
        // Same feature extraction as policy network
        let input_size = 8 * 19 * 19;
        
        let vs = VarBuilder::zeros(dtype, &device);
        
        // Deeper network for position evaluation
        let hidden1 = linear(input_size, 512, vs.pp("h1")).unwrap();
        let hidden2 = linear(512, 512, vs.pp("h2")).unwrap();
        let hidden3 = linear(512, 256, vs.pp("h3")).unwrap();
        let hidden4 = linear(256, 128, vs.pp("h4")).unwrap();
        let output = linear(128, 1, vs.pp("out")).unwrap();
        
        Self {
            input_size,
            hidden1,
            hidden2,
            hidden3,
            hidden4,
            output,
            device,
        }
    }
    
    /// Extract features (same as policy network)
    fn extract_features(&self, game_state: &GameState) -> Tensor {
        // Reuse same feature extraction logic
        let mut features = vec![0.0f32; self.input_size];
        let current_color = game_state.current_turn;
        let board = &game_state.board;
        
        // Feature extraction (same as policy network)
        for y in 0..19 {
            for x in 0..19 {
                let idx = y * 19 + x;
                
                // Current player stones
                if let Some(color) = board.board[y][x] {
                    if color == current_color {
                        features[idx] = 1.0;
                    } else {
                        features[361 + idx] = 1.0;
                    }
                } else {
                    features[2 * 361 + idx] = 1.0;
                }
            }
        }
        
        Tensor::from_vec(features, (self.input_size,), &self.device).unwrap()
    }
    
    /// Evaluate board position
    pub fn evaluate(&self, game_state: &GameState) -> crate::BoardEvaluation {
        let features = self.extract_features(game_state);
        
        // Forward pass
        let x = self.hidden1.forward(&features).unwrap();
        let x = ops::relu(&x).unwrap();
        let x = self.hidden2.forward(&x).unwrap();
        let x = ops::relu(&x).unwrap();
        let x = self.hidden3.forward(&x).unwrap();
        let x = ops::relu(&x).unwrap();
        let x = self.hidden4.forward(&x).unwrap();
        let x = ops::relu(&x).unwrap();
        let value = self.output.forward(&x).unwrap();
        
        // Apply tanh to bound output to [-1, 1]
        let value = ops::tanh(&value).unwrap();
        let win_probability: f32 = value.to_scalar().unwrap();
        
        // Calculate confidence based on game phase
        let move_count = game_state.moves.len();
        let confidence = if move_count < 30 {
            0.3 // Low confidence in opening
        } else if move_count < 100 {
            0.6 // Medium confidence in middle game
        } else {
            0.9 // High confidence in endgame
        };
        
        crate::BoardEvaluation {
            win_probability,
            confidence,
        }
    }
    
    /// Get territory estimation
    pub fn estimate_territory(&self, game_state: &GameState) -> [[f32; 19]; 19] {
        let mut territory = [[0.0; 19]; 19];
        
        // Simple territory estimation based on influence
        for y in 0..19 {
            for x in 0..19 {
                if game_state.board.board[y][x].is_none() {
                    let influence = self.calculate_influence(game_state, x, y);
                    territory[y][x] = influence;
                }
            }
        }
        
        territory
    }
    
    /// Calculate influence at a point (simplified)
    fn calculate_influence(&self, game_state: &GameState, x: usize, y: usize) -> f32 {
        let mut black_influence = 0.0;
        let mut white_influence = 0.0;
        
        // Check surrounding area
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < 19 && ny >= 0 && ny < 19 {
                    if let Some(color) = game_state.board.board[ny as usize][nx as usize] {
                        let distance = ((dx * dx + dy * dy) as f32).sqrt();
                        let influence = 1.0 / (1.0 + distance);
                        
                        match color {
                            Color::Black => black_influence += influence,
                            Color::White => white_influence += influence,
                        }
                    }
                }
            }
        }
        
        // Return normalized influence (-1 for white, +1 for black)
        (black_influence - white_influence) / (black_influence + white_influence + 0.001)
    }
}