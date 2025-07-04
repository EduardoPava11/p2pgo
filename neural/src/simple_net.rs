//! Simple neural network implementation for UI heat maps
//! This provides working neural features without complex dependencies

use p2pgo_core::{GameState, Move, Color, Coord};
use crate::{MovePrediction, BoardEvaluation};
use rand::Rng;
use std::collections::HashMap;

/// Simple pattern-based neural network
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimpleNeuralNet {
    /// Pattern weights for different board positions
    pub pattern_weights: HashMap<String, f32>,
    /// Learning from games
    _game_history: Vec<GameState>,
}

impl SimpleNeuralNet {
    pub fn new() -> Self {
        let mut pattern_weights = HashMap::new();
        
        // Initialize with basic Go patterns
        // Corner patterns (3-3, 4-4, 3-4 points)
        pattern_weights.insert("corner_33".to_string(), 0.8);
        pattern_weights.insert("corner_44".to_string(), 0.85);
        pattern_weights.insert("corner_34".to_string(), 0.75);
        
        // Side patterns
        pattern_weights.insert("side_34".to_string(), 0.7);
        pattern_weights.insert("side_44".to_string(), 0.65);
        
        // Center patterns
        pattern_weights.insert("center_tengen".to_string(), 0.5);
        
        Self {
            pattern_weights,
            _game_history: Vec::new(),
        }
    }
    
    /// Get move predictions with heat map values
    pub fn predict_moves(&self, game_state: &GameState) -> Vec<MovePrediction> {
        let mut predictions = Vec::new();
        let board_size = game_state.board_size as i32;
        
        // Generate predictions for all legal moves
        for x in 0..board_size {
            for y in 0..board_size {
                let coord = Coord::new(x as u8, y as u8);
                
                // Check if move is legal
                let idx = coord.y as usize * board_size as usize + coord.x as usize;
                if idx < game_state.board.len() && game_state.board[idx].is_none() {
                    let _mv = Move::Place {
                        x: x as u8,
                        y: y as u8,
                        color: game_state.current_player,
                    };
                    
                    // Simple legality check - just verify position is empty
                    // A full legality check would include ko and suicide rules
                    if true {
                        let probability = self.evaluate_move(&game_state, coord);
                        predictions.push(MovePrediction {
                            coord,
                            probability,
                        });
                    }
                }
            }
        }
        
        // Normalize probabilities
        let sum: f32 = predictions.iter().map(|p| p.probability).sum();
        if sum > 0.0 {
            for pred in &mut predictions {
                pred.probability /= sum;
            }
        }
        
        // Sort by probability (best moves first)
        predictions.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap());
        
        predictions
    }
    
    /// Evaluate a specific move
    fn evaluate_move(&self, game_state: &GameState, coord: Coord) -> f32 {
        let mut score = 0.1; // Base score for legal moves
        
        let x = coord.x as i32;
        let y = coord.y as i32;
        let board_size = game_state.board_size as i32;
        
        // Corner evaluation (first 4 moves)
        if game_state.moves.len() < 4 {
            // 3-3 points
            if (x == 2 || x == board_size - 3) && (y == 2 || y == board_size - 3) {
                score += self.pattern_weights.get("corner_33").unwrap_or(&0.0);
            }
            // 4-4 points
            if (x == 3 || x == board_size - 4) && (y == 3 || y == board_size - 4) {
                score += self.pattern_weights.get("corner_44").unwrap_or(&0.0);
            }
            // 3-4 points
            if (x == 2 && (y == 3 || y == board_size - 4)) ||
                (x == 3 && (y == 2 || y == board_size - 3)) ||
                (x == board_size - 3 && (y == 3 || y == board_size - 4)) ||
                (x == board_size - 4 && (y == 2 || y == board_size - 3)) {
                score += self.pattern_weights.get("corner_34").unwrap_or(&0.0);
            }
        }
        
        // Proximity to existing stones (local play)
        for dx in -2..=2 {
            for dy in -2..=2 {
                if dx == 0 && dy == 0 { continue; }
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && nx < board_size && ny >= 0 && ny < board_size {
                    let idx = ny as usize * board_size as usize + nx as usize;
                    if idx < game_state.board.len() && game_state.board[idx].is_some() {
                        // Closer stones have more influence
                        let distance = (dx.abs() + dy.abs()) as f32;
                        score += 0.2 / distance;
                    }
                }
            }
        }
        
        // Encourage extending from friendly stones
        for dx in [-1, 0, 1] {
            for dy in [-1, 0, 1] {
                if dx == 0 && dy == 0 { continue; }
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && nx < board_size && ny >= 0 && ny < board_size {
                    let idx = ny as usize * board_size as usize + nx as usize;
                    if idx < game_state.board.len() {
                        if let Some(color) = game_state.board[idx] {
                            if color == game_state.current_player {
                                score += 0.3; // Extend from friendly stones
                            } else {
                                score += 0.2; // Contact plays can be good too
                            }
                        }
                    }
                }
            }
        }
        
        // Avoid playing on the first line early
        if game_state.moves.len() < 20 && (x == 0 || x == board_size - 1 || y == 0 || y == board_size - 1) {
            score *= 0.3;
        }
        
        // Add some randomness for variety
        let mut rng = rand::thread_rng();
        score += rng.gen_range(0.0..0.1);
        
        score.max(0.01) // Ensure positive probability
    }
    
    /// Evaluate board position
    pub fn evaluate_position(&self, game_state: &GameState) -> BoardEvaluation {
        // Simple evaluation based on territory estimation
        let mut black_territory = 0;
        let mut white_territory = 0;
        
        let board_size = game_state.board_size;
        
        // Count stones and estimate influence
        for x in 0..board_size {
            for y in 0..board_size {
                let coord = Coord::new(x, y);
                let idx = y as usize * board_size as usize + x as usize;
                match game_state.board.get(idx) {
                    Some(&Some(Color::Black)) => black_territory += 1,
                    Some(&Some(Color::White)) => white_territory += 1,
                    Some(&None) | None => {
                        // Estimate influence for empty points
                        let (black_influence, white_influence) = self.calculate_influence(game_state, coord);
                        if black_influence > white_influence * 1.5 {
                            black_territory += 1;
                        } else if white_influence > black_influence * 1.5 {
                            white_territory += 1;
                        }
                    }
                }
            }
        }
        
        // Add captured stones
        black_territory += game_state.captures.1 as i32; // White stones captured by black
        white_territory += game_state.captures.0 as i32; // Black stones captured by white
        
        // Calculate win probability (-1 to 1)
        let total = (black_territory + white_territory) as f32;
        let black_ratio = black_territory as f32 / total.max(1.0);
        let white_ratio = white_territory as f32 / total.max(1.0);
        
        let win_probability = match game_state.current_player {
            Color::Black => (black_ratio - white_ratio).clamp(-1.0, 1.0),
            Color::White => (white_ratio - black_ratio).clamp(-1.0, 1.0),
        };
        
        // Confidence based on game progress
        let confidence = (game_state.moves.len() as f32 / 100.0).min(0.9);
        
        BoardEvaluation {
            win_probability,
            confidence,
        }
    }
    
    /// Calculate influence at a point
    fn calculate_influence(&self, game_state: &GameState, coord: Coord) -> (f32, f32) {
        let mut black_influence = 0.0;
        let mut white_influence = 0.0;
        
        let board_size = game_state.board_size as i32;
        let x = coord.x as i32;
        let y = coord.y as i32;
        
        // Check nearby stones (up to 4 points away)
        for dx in -4..=4 {
            for dy in -4..=4 {
                if dx == 0 && dy == 0 { continue; }
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && nx < board_size && ny >= 0 && ny < board_size {
                    let idx = ny as usize * board_size as usize + nx as usize;
                    if idx < game_state.board.len() {
                        if let Some(color) = game_state.board[idx] {
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
        }
        
        (black_influence, white_influence)
    }
}

/// Training data for neural network
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingData {
    pub game_states: Vec<GameState>,
    pub moves: Vec<Move>,
    pub winner: Option<Color>,
}

impl TrainingData {
    pub fn new() -> Self {
        Self {
            game_states: Vec::new(),
            moves: Vec::new(),
            winner: None,
        }
    }
    
    pub fn add_position(&mut self, state: GameState, mv: Move) {
        self.game_states.push(state);
        self.moves.push(mv);
    }
    
    pub fn set_winner(&mut self, winner: Color) {
        self.winner = Some(winner);
    }
    
    /// Get states (alias for game_states)
    pub fn states(&self) -> &[GameState] {
        &self.game_states
    }
}