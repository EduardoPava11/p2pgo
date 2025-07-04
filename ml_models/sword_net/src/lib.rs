//! Sword Net - Lightweight Aggressive Go Policy Model
//!
//! This is a CPU-only implementation that demonstrates the interface
//! for aggressive Go policy models without heavy ML dependencies.
//! Supports WASM compilation and CBOR I/O for deterministic cross-platform operation.

use serde::{Deserialize, Serialize};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
// Note: These imports will be used when implementing real neural network inference
// use ndarray::{Array1, Array3};
// use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Network configuration for Sword Net
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwordNetConfig {
    /// Board size (9 for standard game)
    pub board_size: usize,
    
    /// Number of input channels (3: black stones, white stones, empty)
    pub input_channels: usize,
    
    /// Random seed for reproducible behavior
    pub seed: u64,
}

impl Default for SwordNetConfig {
    fn default() -> Self {
        Self {
            board_size: 9,
            input_channels: 3,
            seed: 12345,
        }
    }
}

/// Lightweight Sword Net implementation
pub struct SwordNet {
    config: SwordNetConfig,
    rng: StdRng,
}

impl SwordNet {
    /// Create a new Sword Net
    pub fn new(config: SwordNetConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);
        Self { config, rng }
    }
    
    /// Inference on a single board state
    pub fn infer(&mut self, board: &[f32]) -> Vec<f32> {
        assert_eq!(board.len(), self.config.board_size * self.config.board_size * self.config.input_channels);
        sword_inference_impl(board, &mut self.rng)
    }
}

/// WASM-compatible inference function
/// Input: pointer to board state (81 * 3 floats)
/// Output: pointer to probabilities (81 floats)
#[no_mangle]
pub extern "C" fn sword_infer(board_ptr: *const f32, board_len: usize, out_ptr: *mut f32) -> i32 {
    if board_ptr.is_null() || out_ptr.is_null() {
        return -1; // Error: null pointer
    }
    
    let expected_len = 9 * 9 * 3; // 9x9 board with 3 channels
    if board_len != expected_len {
        return -2; // Error: invalid input size
    }
    
    let board_slice = unsafe { std::slice::from_raw_parts(board_ptr, board_len) };
    let mut rng = StdRng::seed_from_u64(12345); // Fixed seed for consistency
    let probabilities = sword_inference_impl(board_slice, &mut rng);
    
    // Copy results to output buffer
    unsafe {
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, 81);
        out_slice.copy_from_slice(&probabilities);
    }
    
    0 // Success
}

/// Core inference implementation
fn sword_inference_impl(board: &[f32], rng: &mut StdRng) -> Vec<f32> {
    let mut probabilities = vec![0.01f32; 81]; // Base probability for all positions
    let board_size = 9;
    
    // Sword strategy: prefer corners and edges (aggressive territorial expansion)
    let corner_positions = [0, 8, 72, 80]; // Corners of 9x9 board
    let edge_positions = [
        1, 2, 3, 4, 5, 6, 7,           // Top edge
        9, 17, 25, 33, 41, 49, 57, 65, // Left edge  
        15, 23, 31, 39, 47, 55, 63, 71, // Right edge
        73, 74, 75, 76, 77, 78, 79,    // Bottom edge
    ];
    
    // Check if positions are empty and boost probability
    for &pos in &corner_positions {
        if pos < 81 {
            let channel_offset = pos * 3;
            if channel_offset + 2 < board.len() {
                let is_empty = board[channel_offset] == 0.0 && board[channel_offset + 1] == 0.0;
                if is_empty {
                    probabilities[pos] = 0.15 + rng.gen::<f32>() * 0.05; // High probability for empty corners
                }
            }
        }
    }
    
    for &pos in &edge_positions {
        if pos < 81 {
            let channel_offset = pos * 3;
            if channel_offset + 2 < board.len() {
                let is_empty = board[channel_offset] == 0.0 && board[channel_offset + 1] == 0.0;
                if is_empty {
                    probabilities[pos] = 0.08 + rng.gen::<f32>() * 0.03; // Medium probability for empty edges
                }
            }
        }
    }
    
    // Add some aggressive patterns: favor moves that attack opponent groups
    for y in 0..board_size {
        for x in 0..board_size {
            let pos = y * board_size + x;
            let channel_offset = pos * 3;
            
            if channel_offset + 2 < board.len() {
                let is_empty = board[channel_offset] == 0.0 && board[channel_offset + 1] == 0.0;
                
                if is_empty {
                    // Check for adjacent opponent stones to attack
                    let attack_bonus = calculate_attack_bonus(board, x, y, board_size);
                    probabilities[pos] += attack_bonus;
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

/// Calculate bonus for attacking moves
fn calculate_attack_bonus(board: &[f32], x: usize, y: usize, board_size: usize) -> f32 {
    let mut bonus = 0.0;
    
    // Check adjacent positions for opponent stones
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
                    bonus += 0.05; // Bonus for attacking opponent stones
                }
            }
        }
    }
    
    bonus
}

/// CBOR-based board state structure for deterministic I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CborBoardState {
    /// Board size (typically 9)
    pub size: u8,
    /// Flattened board state: 0=empty, 1=black, 2=white
    pub stones: Vec<u8>,
    /// Player to move: 0=black, 1=white  
    pub next_player: u8,
    /// Capture counts [black_captures, white_captures]
    pub captures: [u16; 2],
    /// Previous state hash (for ko detection)
    pub previous_state: Option<Vec<u8>>,
}

/// CBOR-based inference result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CborInferenceResult {
    /// Move probabilities for each position (81 values for 9x9)
    pub probabilities: Vec<f32>,
    /// Value estimate [-1.0, 1.0] where 1.0 = winning for next_player
    pub value: f32,
    /// Model metadata
    pub model_id: u32,
    /// Inference timestamp
    pub timestamp: u64,
}

/// CBOR-based inference function for WASM
#[no_mangle]
pub extern "C" fn sword_infer_cbor(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr_ptr: *mut *mut u8,
    output_len_ptr: *mut usize,
) -> i32 {
    if input_ptr.is_null() || output_ptr_ptr.is_null() || output_len_ptr.is_null() {
        return -1; // Error: null pointer
    }
    
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    
    // Deserialize CBOR input
    let board_state: CborBoardState = match ciborium::de::from_reader(input_slice) {
        Ok(state) => state,
        Err(_) => return -2, // Error: invalid CBOR
    };
    
    // Convert board state to inference format
    let board_input = convert_board_to_inference_input(&board_state);
    
    // Run inference
    let mut rng = StdRng::seed_from_u64(12345); // Deterministic seed
    let probabilities = sword_inference_impl(&board_input, &mut rng);
    
    // Calculate value estimate (simplified)
    let value = calculate_position_value(&board_state, &probabilities);
    
    // Create result
    let result = CborInferenceResult {
        probabilities,
        value,
        model_id: sword_get_model_id(),
        timestamp: get_timestamp(),
    };
    
    // Serialize result to CBOR
    let mut output_buffer = Vec::new();
    if ciborium::ser::into_writer(&result, &mut output_buffer).is_err() {
        return -3; // Error: serialization failed
    }
    
    // Allocate output buffer and copy data
    let output_len = output_buffer.len();
    let output_ptr = unsafe { 
        let ptr = std::alloc::alloc(std::alloc::Layout::from_size_align(output_len, 1).unwrap());
        std::ptr::copy_nonoverlapping(output_buffer.as_ptr(), ptr, output_len);
        ptr
    };
    
    unsafe {
        *output_ptr_ptr = output_ptr;
        *output_len_ptr = output_len;
    }
    
    0 // Success
}

/// Convert CBOR board state to inference input format
fn convert_board_to_inference_input(board_state: &CborBoardState) -> Vec<f32> {
    let size = board_state.size as usize;
    let mut input = vec![0.0f32; size * size * 3];
    
    for (i, &stone) in board_state.stones.iter().enumerate() {
        let channel_offset = i * 3;
        match stone {
            1 => input[channel_offset] = 1.0,     // Black stone
            2 => input[channel_offset + 1] = 1.0, // White stone
            _ => input[channel_offset + 2] = 1.0, // Empty
        }
    }
    
    input
}

/// Calculate position value estimate
fn calculate_position_value(board_state: &CborBoardState, probabilities: &[f32]) -> f32 {
    // Simplified value calculation based on:
    // 1. Material advantage (captures)
    // 2. Territory potential (high probability moves)
    // 3. Positional factors
    
    let black_captures = board_state.captures[0] as f32;
    let white_captures = board_state.captures[1] as f32;
    let material_advantage = (black_captures - white_captures) * 0.1;
    
    // Territory potential based on move probabilities
    let max_prob = probabilities.iter().fold(0.0f32, |a, &b| a.max(b));
    let territory_potential = (max_prob - 0.1).max(0.0) * 2.0;
    
    // Combine factors (value from black's perspective)
    let value = material_advantage + territory_potential;
    
    // Adjust for current player
    if board_state.next_player == 0 {
        value.tanh() // Black to play
    } else {
        (-value).tanh() // White to play
    }
}

/// Get current timestamp (mock implementation for WASM)
fn get_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, use performance.now() or similar
        0 // Placeholder
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Get model metadata
#[no_mangle]
pub extern "C" fn sword_get_model_id() -> u32 {
    1001 // Sword Net model ID
}

#[no_mangle]
pub extern "C" fn sword_get_version() -> u32 {
    100 // Version 1.0.0
}

/// Free memory allocated by sword_infer_cbor
#[no_mangle]
pub extern "C" fn sword_free_result(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            std::alloc::dealloc(ptr, std::alloc::Layout::from_size_align(len, 1).unwrap());
        }
    }
}

/// Training step (placeholder for federated learning)
#[no_mangle]
pub extern "C" fn sword_train_step(
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
    fn test_sword_config() {
        let config = SwordNetConfig::default();
        assert_eq!(config.board_size, 9);
        assert_eq!(config.input_channels, 3);
    }
    
    #[test]
    fn test_sword_net() {
        let config = SwordNetConfig::default();
        let mut net = SwordNet::new(config);
        
        let empty_board = vec![0.0f32; 9 * 9 * 3];
        let probabilities = net.infer(&empty_board);
        
        assert_eq!(probabilities.len(), 81);
        
        // Check that probabilities sum to approximately 1.0
        let sum: f32 = probabilities.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
        
        // Check that corners have higher probability than center
        let corner_prob = probabilities[0]; // Top-left corner
        let center_prob = probabilities[40]; // Center position (4,4)
        assert!(corner_prob > center_prob);
    }
    
    #[test]
    fn test_wasm_interface() {
        let board = vec![0.0f32; 9 * 9 * 3];
        let mut output = vec![0.0f32; 81];
        
        let result = sword_infer(
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
    fn test_attack_bonus() {
        let mut board = vec![0.0f32; 9 * 9 * 3];
        
        // Place a white stone at position (4, 4)
        let pos = 4 * 9 + 4;
        board[pos * 3 + 1] = 1.0; // White stone
        
        // Check bonus for adjacent position (3, 4)
        let bonus = calculate_attack_bonus(&board, 4, 3, 9);
        assert!(bonus > 0.0);
    }
}