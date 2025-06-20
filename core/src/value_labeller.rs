// SPDX-License-Identifier: MIT OR Apache-2.0

//! Value-head labelling for training data with final scores

use crate::{GameState, Color, Coord};
use serde::{Serialize, Deserialize};

/// Proof of a game's final score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreProof {
    pub final_score: i16, // Positive for Black, negative for White
    pub territory_black: u16,
    pub territory_white: u16,
    pub captures_black: u16,
    pub captures_white: u16,
    pub komi: f32,
    pub method: ScoringMethod,
}

/// Method used to determine the final score
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScoringMethod {
    /// Territory counting (Chinese rules)
    Territory,
    /// Area counting (Japanese rules)
    Area,
    /// Resignation by specified player
    Resignation(Color),
    /// Time forfeit by specified player
    TimeOut(Color),
}

/// Value label for a move position in training data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueLabel {
    pub move_number: u32,
    pub position_value: f32, // -1.0 to 1.0, from current player's perspective
    pub game_outcome: f32,   // Final game result from current player's perspective
    pub confidence: f32,     // 0.0 to 1.0, confidence in the labelling
}

/// Value-head labeller for generating training labels
pub struct ValueLabeller {
    move_values: Vec<ValueLabel>,
    final_score: Option<ScoreProof>,
}

impl ValueLabeller {
    /// Create a new value labeller
    pub fn new() -> Self {
        Self {
            move_values: Vec::new(),
            final_score: None,
        }
    }
    
    /// Set the final score proof for the game
    pub fn set_final_score(&mut self, score_proof: ScoreProof) {
        self.final_score = Some(score_proof);
        self.recalculate_all_values();
    }
    
    /// Add a move position for labelling
    pub fn add_move_position(&mut self, move_number: u32, game_state: &GameState) {
        let position_value = self.estimate_position_value(game_state);
        
        let label = ValueLabel {
            move_number,
            position_value,
            game_outcome: 0.0, // Will be set when final score is known
            confidence: 0.8,   // Default confidence
        };
        
        self.move_values.push(label);
    }
    
    /// Get value label for a specific move
    pub fn get_value_label(&self, move_number: u32) -> Option<&ValueLabel> {
        self.move_values.iter().find(|v| v.move_number == move_number)
    }
    
    /// Get all value labels
    pub fn get_all_labels(&self) -> &[ValueLabel] {
        &self.move_values
    }
    
    /// Export labels for training
    pub fn export_training_data(&self) -> Vec<u8> {
        match serde_cbor::to_vec(&self.move_values) {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to serialize value labels: {}", e);
                Vec::new()
            }
        }
    }
    
    /// Estimate position value (simplified heuristic)
    fn estimate_position_value(&self, game_state: &GameState) -> f32 {
        // Simple heuristic based on stone count and territory estimation
        let mut black_score = 0;
        let mut white_score = 0;
        
        // Count stones
        for stone in &game_state.board {
            match stone {
                Some(Color::Black) => black_score += 1,
                Some(Color::White) => white_score += 1,
                None => {}
            }
        }
        
        // Simple territory estimation (empty spaces surrounded by same color)
        let territory_estimate = self.estimate_territory(game_state);
        black_score += territory_estimate.0;
        white_score += territory_estimate.1;
        
        // Convert to value from current player's perspective
        let score_diff = match game_state.current_player {
            Color::Black => black_score - white_score,
            Color::White => white_score - black_score,
        };
        
        // Normalize to -1.0 to 1.0 range
        let max_possible_score = (game_state.board_size as i32).pow(2);
        (score_diff as f32) / (max_possible_score as f32)
    }
    
    /// Simple territory estimation
    fn estimate_territory(&self, game_state: &GameState) -> (i32, i32) {
        let mut black_territory = 0;
        let mut white_territory = 0;
        
        for y in 0..game_state.board_size {
            for x in 0..game_state.board_size {
                let coord = Coord { x, y };
                let idx = (y as usize) * (game_state.board_size as usize) + (x as usize);
                
                if game_state.board[idx].is_none() {
                    // Empty point - estimate which territory it belongs to
                    let surrounding = self.get_surrounding_influence(game_state, coord);
                    if surrounding.0 > surrounding.1 {
                        black_territory += 1;
                    } else if surrounding.1 > surrounding.0 {
                        white_territory += 1;
                    }
                }
            }
        }
        
        (black_territory, white_territory)
    }
    
    /// Get influence of surrounding stones
    fn get_surrounding_influence(&self, game_state: &GameState, coord: Coord) -> (i32, i32) {
        let mut black_influence: i32 = 0;
        let mut white_influence: i32 = 0;
        
        // Check in expanding squares around the point
        for radius in 1..=3 {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    if dx == 0 && dy == 0 { continue; }
                    
                    let check_x = coord.x as i8 + dx;
                    let check_y = coord.y as i8 + dy;
                    
                    if check_x >= 0 && check_x < game_state.board_size as i8 &&
                       check_y >= 0 && check_y < game_state.board_size as i8 {
                        let idx = (check_y as usize) * (game_state.board_size as usize) + (check_x as usize);
                        
                        match game_state.board.get(idx).and_then(|c| *c) {
                            Some(Color::Black) => black_influence += (4 - radius) as i32, // Closer stones have more influence
                            Some(Color::White) => white_influence += (4 - radius) as i32,
                            None => {}
                        }
                    }
                }
            }
        }
        
        (black_influence, white_influence)
    }
    
    /// Recalculate all position values based on final score
    fn recalculate_all_values(&mut self) {
        if let Some(ref score_proof) = self.final_score {
            let final_outcome = match score_proof.method {
                ScoringMethod::Territory | ScoringMethod::Area => {
                    // Use actual score
                    let normalized_score = (score_proof.final_score as f32) / 100.0; // Normalize
                    normalized_score.clamp(-1.0, 1.0)
                }
                ScoringMethod::Resignation(winner) | ScoringMethod::TimeOut(winner) => {
                    // Win/loss
                    match winner {
                        Color::Black => 1.0,
                        Color::White => -1.0,
                    }
                }
            };
            
            // Update all move values with the known outcome
            for label in &mut self.move_values {
                // Alternate perspective based on move number
                let player_perspective = if label.move_number % 2 == 0 {
                    -final_outcome // White's perspective
                } else {
                    final_outcome  // Black's perspective
                };
                
                label.game_outcome = player_perspective;
                
                // Increase confidence for positions closer to the final outcome
                let position_accuracy = 1.0 - (label.position_value - player_perspective).abs();
                label.confidence = (0.5 + 0.5 * position_accuracy).clamp(0.0, 1.0);
            }
        }
    }
    
    /// Label a game's moves and persist value labels with score proof
    /// This function is used to create training data for the neural network
    pub fn label_and_persist(&mut self, game_state: &GameState, score_proof: ScoreProof) -> Vec<u8> {
        // First add all move positions to the labeller
        for i in 0..game_state.moves.len() {
            // Reconstruct game state at each move
            let mut historical_state = GameState::new(game_state.board_size);
            for j in 0..=i {
                if j < game_state.moves.len() {
                    let _ = historical_state.apply_move(game_state.moves[j].clone());
                }
            }
            // Add the move position to the labeller
            self.add_move_position(i as u32, &historical_state);
        }
        
        // Set the final score to calculate game outcomes
        self.set_final_score(score_proof.clone());
        
        // Create a buffer for serialized data
        let mut buffer = Vec::new();
        
        // Add score proof with 'S' marker
        buffer.push(b'S');
        if let Ok(score_data) = serde_cbor::to_vec(&score_proof) {
            buffer.extend(score_data);
        }
        
        // Add move labels with 'M' marker
        buffer.push(b'M');
        buffer.extend(self.export_training_data());
        
        buffer
    }
}

impl Default for ValueLabeller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_value_labeller() {
        let mut labeller = ValueLabeller::new();
        let game_state = GameState::new(9);
        
        // Add some move positions
        labeller.add_move_position(1, &game_state);
        labeller.add_move_position(2, &game_state);
        
        assert_eq!(labeller.get_all_labels().len(), 2);
        
        // Set final score
        let score_proof = ScoreProof {
            final_score: 15,
            territory_black: 40,
            territory_white: 35,
            captures_black: 2,
            captures_white: 1,
            komi: 6.5,
            method: ScoringMethod::Territory,
        };
        
        labeller.set_final_score(score_proof);
        
        // Check that outcomes were updated
        for label in labeller.get_all_labels() {
            assert_ne!(label.game_outcome, 0.0);
        }
    }
    
    #[test]
    fn test_score_proof_serialization() {
        let score_proof = ScoreProof {
            final_score: -7,
            territory_black: 30,
            territory_white: 40,
            captures_black: 0,
            captures_white: 3,
            komi: 6.5,
            method: ScoringMethod::Resignation(Color::White),
        };
        
        let serialized = serde_cbor::to_vec(&score_proof).unwrap();
        let deserialized: ScoreProof = serde_cbor::from_slice(&serialized).unwrap();
        
        assert_eq!(deserialized.final_score, -7);
        match deserialized.method {
            ScoringMethod::Resignation(Color::White) => {},
            _ => panic!("Wrong scoring method"),
        }
    }
}
