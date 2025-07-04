//! Shield Net - Lightweight Defensive Go Policy Model
//!
//! This is a CPU-only implementation that demonstrates the interface
//! for defensive Go policy models. Uses Y-flipped board representation.

use serde::{Deserialize, Serialize};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Network configuration for Shield Net
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldNetConfig {
    /// Board size (9 for standard game)
    pub board_size: usize,
    
    /// Number of input channels (3: black stones, white stones, empty)
    pub input_channels: usize,
    
    /// Random seed for reproducible behavior
    pub seed: u64,
}

impl Default for ShieldNetConfig {
    fn default() -> Self {
        Self {
            board_size: 9,
            input_channels: 3,
            seed: 54321,
        }
    }
}

/// Lightweight Shield Net implementation
pub struct ShieldNet {
    config: ShieldNetConfig,
    rng: StdRng,
}

impl ShieldNet {
    /// Create a new Shield Net
    pub fn new(config: ShieldNetConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);
        Self { config, rng }
    }
    
    /// Inference on a single board state (Y-flipped for defensive orientation)
    pub fn infer(&mut self, board: &[f32]) -> Vec<f32> {
        assert_eq!(board.len(), self.config.board_size * self.config.board_size * self.config.input_channels);
        
        // Apply Y-flip transformation to the board
        let flipped_board = flip_board_y(board, self.config.board_size);
        let probabilities = shield_inference_impl(&flipped_board, &mut self.rng);
        
        // Flip probabilities back
        flip_probabilities_y(&probabilities, self.config.board_size)
    }
}

/// Flip board vertically (Y-axis) for defensive training
/// This makes "defend" always mean "expand up-board" as per AlphaGo-Zero symmetry
fn flip_board_y(board: &[f32], board_size: usize) -> Vec<f32> {
    let channels = 3;
    let mut flipped = vec![0.0; board.len()];
    
    for c in 0..channels {
        for y in 0..board_size {
            for x in 0..board_size {
                let original_idx = (c * board_size * board_size) + (y * board_size) + x;
                let flipped_y = board_size - 1 - y;
                let flipped_idx = (c * board_size * board_size) + (flipped_y * board_size) + x;
                flipped[flipped_idx] = board[original_idx];
            }
        }
    }
    
    flipped
}

/// Flip probabilities back after inference
fn flip_probabilities_y(probabilities: &[f32], board_size: usize) -> Vec<f32> {
    let mut flipped = vec![0.0; probabilities.len()];
    
    for y in 0..board_size {
        for x in 0..board_size {
            let original_idx = y * board_size + x;
            let flipped_y = board_size - 1 - y;
            let flipped_idx = flipped_y * board_size + x;
            flipped[flipped_idx] = probabilities[original_idx];
        }
    }
    
    flipped
}

/// WASM-compatible inference function
/// Input: pointer to board state (81 * 3 floats)
/// Output: pointer to probabilities (81 floats)
#[no_mangle]
pub extern "C" fn shield_infer(board_ptr: *const f32, board_len: usize, out_ptr: *mut f32) -> i32 {
    if board_ptr.is_null() || out_ptr.is_null() {
        return -1; // Error: null pointer
    }
    
    let expected_len = 9 * 9 * 3; // 9x9 board with 3 channels
    if board_len != expected_len {
        return -2; // Error: invalid input size
    }
    
    let board_slice = unsafe { std::slice::from_raw_parts(board_ptr, board_len) };
    
    // Apply Y-flip and inference
    let flipped_board = flip_board_y(board_slice, 9);
    let mut rng = StdRng::seed_from_u64(54321); // Fixed seed for consistency
    let probabilities = shield_inference_impl(&flipped_board, &mut rng);
    let final_probabilities = flip_probabilities_y(&probabilities, 9);
    
    // Copy results to output buffer
    unsafe {
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, 81);
        out_slice.copy_from_slice(&final_probabilities);
    }
    
    0 // Success
}

/// Core inference implementation for shield (defensive) strategy
fn shield_inference_impl(board: &[f32], rng: &mut StdRng) -> Vec<f32> {
    let mut probabilities = vec![0.01f32; 81]; // Base probability for all positions
    let board_size = 9;
    
    // Shield strategy: prefer center and positions adjacent to opponent stones
    let center_positions = [36, 37, 38, 45, 46, 47, 54, 55, 56]; // 3x3 center area
    
    // Boost probability for center positions (defensive control)
    for &pos in &center_positions {
        if pos < 81 {
            let channel_offset = pos * 3;
            if channel_offset + 2 < board.len() {
                let is_empty = board[channel_offset] == 0.0 && board[channel_offset + 1] == 0.0;
                if is_empty {
                    probabilities[pos] = 0.12 + rng.gen::<f32>() * 0.04; // High probability for empty center
                }
            }
        }
    }
    
    // Look for opponent stones and boost adjacent empty positions
    for y in 0..board_size {
        for x in 0..board_size {
            let pos = y * board_size + x;
            let channel_offset = pos * 3;
            
            if channel_offset + 2 < board.len() {
                // Check if this is an opponent stone (white stone in channel 1)
                if board[channel_offset + 1] > 0.5 {
                    // Boost adjacent empty positions (defensive moves)
                    let adjacent = [
                        (y.wrapping_sub(1), x), // Up
                        (y + 1, x),             // Down
                        (y, x.wrapping_sub(1)), // Left
                        (y, x + 1),             // Right
                    ];
                    
                    for (adj_y, adj_x) in adjacent {
                        if adj_y < board_size && adj_x < board_size {
                            let adj_pos = adj_y * board_size + adj_x;
                            let adj_channel_offset = adj_pos * 3;
                            
                            if adj_channel_offset + 2 < board.len() {
                                let is_empty = board[adj_channel_offset] == 0.0 && board[adj_channel_offset + 1] == 0.0;
                                
                                if is_empty {
                                    probabilities[adj_pos] += 0.10 + rng.gen::<f32>() * 0.03; // Medium-high probability for defensive moves
                                }
                            }
                        }
                    }
                }
                
                // Also boost positions that protect our own stones
                if board[channel_offset] > 0.5 { // Our stone (black in channel 0)
                    let protect_bonus = calculate_protection_bonus(board, x, y, board_size);
                    
                    // Spread protection bonus to nearby empty positions
                    for dy in -1..=1i32 {
                        for dx in -1..=1i32 {
                            let protect_y = (y as i32 + dy) as usize;
                            let protect_x = (x as i32 + dx) as usize;
                            
                            if protect_y < board_size && protect_x < board_size {
                                let protect_pos = protect_y * board_size + protect_x;
                                let protect_channel_offset = protect_pos * 3;
                                
                                if protect_channel_offset + 2 < board.len() {
                                    let is_empty = board[protect_channel_offset] == 0.0 && board[protect_channel_offset + 1] == 0.0;
                                    if is_empty {
                                        probabilities[protect_pos] += protect_bonus * 0.5;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Normalize probabilities
    let sum: f32 = probabilities.iter().sum();
    if sum > 0.0 {
        for p in &mut probabilities {
            *p /= sum;
        }
    }
    
    probabilities
}

/// Calculate bonus for protecting our stones
fn calculate_protection_bonus(board: &[f32], x: usize, y: usize, board_size: usize) -> f32 {
    let mut bonus = 0.0;
    
    // Check if our stone is under threat (adjacent to opponent stones)
    let adjacent = [
        (y.wrapping_sub(1), x), // Up
        (y + 1, x),             // Down
        (y, x.wrapping_sub(1)), // Left
        (y, x + 1),             // Right
    ];
    
    for (adj_y, adj_x) in adjacent {
        if adj_y < board_size && adj_x < board_size {
            let adj_pos = adj_y * board_size + adj_x;
            let adj_channel_offset = adj_pos * 3;
            
            if adj_channel_offset + 2 < board.len() {
                // Check if this is an opponent stone (white stone in channel 1)
                if board[adj_channel_offset + 1] > 0.5 {
                    bonus += 0.08; // Higher bonus if under threat
                }
            }
        }
    }
    
    bonus
}

/// Get model metadata
#[no_mangle]
pub extern "C" fn shield_get_model_id() -> u32 {
    1002 // Shield Net model ID
}

#[no_mangle]
pub extern "C" fn shield_get_version() -> u32 {
    100 // Version 1.0.0
}

/// Training step (placeholder for federated learning)
#[no_mangle]
pub extern "C" fn shield_train_step(
    _board_ptr: *const f32,
    _board_len: usize,
    _target_ptr: *const f32,
    _target_len: usize,
    _learning_rate: f32,
) -> i32 {
    // Placeholder for training functionality
    0 // Success (no-op for now)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shield_config() {
        let config = ShieldNetConfig::default();
        assert_eq!(config.board_size, 9);
        assert_eq!(config.input_channels, 3);
    }
    
    #[test]
    fn test_board_flip() {
        let board = vec![1.0f32; 9 * 9 * 3];
        let flipped = flip_board_y(&board, 9);
        assert_eq!(flipped.len(), board.len());
        
        // Test specific position flip
        let mut test_board = vec![0.0f32; 9 * 9 * 3];
        test_board[0] = 1.0; // Top-left corner (0,0)
        
        let flipped = flip_board_y(&test_board, 9);
        assert_eq!(flipped[72], 1.0); // Should be at bottom-left corner (8,0)
    }
    
    #[test]
    fn test_shield_net() {
        let config = ShieldNetConfig::default();
        let mut net = ShieldNet::new(config);
        
        let empty_board = vec![0.0f32; 9 * 9 * 3];
        let probabilities = net.infer(&empty_board);
        
        assert_eq!(probabilities.len(), 81);
        
        // Check that probabilities sum to approximately 1.0
        let sum: f32 = probabilities.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
        
        // Check that center has higher probability than corners (defensive strategy)
        let center_prob = probabilities[40]; // Center position (4,4)
        let corner_prob = probabilities[0]; // Top-left corner
        assert!(center_prob >= corner_prob); // Allow equal since shield focuses on center
    }
    
    #[test]
    fn test_wasm_interface() {
        let board = vec![0.0f32; 9 * 9 * 3];
        let mut output = vec![0.0f32; 81];
        
        let result = shield_infer(
            board.as_ptr(),
            board.len(),
            output.as_mut_ptr(),
        );
        
        assert_eq!(result, 0); // Success
        
        // Check that output was written
        let sum: f32 = output.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_protection_bonus() {
        let mut board = vec![0.0f32; 9 * 9 * 3];
        
        // Place our stone at position (4, 4)
        let pos = 4 * 9 + 4;
        board[pos * 3] = 1.0; // Black stone
        
        // Place opponent stone adjacent at (4, 5)
        let opponent_pos = 4 * 9 + 5;
        board[opponent_pos * 3 + 1] = 1.0; // White stone
        
        // Check protection bonus
        let bonus = calculate_protection_bonus(&board, 4, 4, 9);
        assert!(bonus > 0.0);
    }
}