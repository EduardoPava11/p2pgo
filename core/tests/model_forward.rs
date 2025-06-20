// SPDX-License-Identifier: MIT OR Apache-2.0

//! Model forward pass tests

/// Mock GoMini6E model for testing
pub struct GoMini6E {
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
}

impl GoMini6E {
    pub fn new() -> Self {
        Self {
            input_size: 81,
            hidden_size: 64,
            output_size: 81,
        }
    }
    
    pub fn forward(&self, input: &[f32]) -> (Vec<f32>, f32) {
        assert_eq!(input.len(), self.input_size);
        
        // Mock forward pass: policy logits and value scalar
        let policy_logits: Vec<f32> = (0..self.output_size)
            .map(|i| (i as f32 * 0.1) % 1.0)
            .collect();
        
        let value_scalar = input.iter().sum::<f32>() / input.len() as f32;
        
        (policy_logits, value_scalar)
    }
}

#[test]
fn test_model_instantiation() {
    let model = GoMini6E::new();
    assert_eq!(model.input_size, 81);
    assert_eq!(model.output_size, 81);
}

#[test]
fn test_model_forward_shapes() {
    let model = GoMini6E::new();
    let input: Vec<f32> = (0..81).map(|i| (i as f32) / 81.0).collect();
    
    let (policy_logits, value_scalar) = model.forward(&input);
    
    // Check output dimensions
    assert_eq!(policy_logits.len(), 81); // (batch=1, moves=81)
    assert!(value_scalar.is_finite()); // Single scalar value
}

#[test]
fn test_model_batch_forward() {
    let model = GoMini6E::new();
    let batch_size = 4;
    
    for _ in 0..batch_size {
        let input: Vec<f32> = (0..81).map(|i| (i as f32) / 81.0).collect();
        let (policy_logits, value_scalar) = model.forward(&input);
        
        assert_eq!(policy_logits.len(), 81);
        assert!(value_scalar.is_finite());
    }
}

#[test]
fn test_model_different_inputs() {
    let model = GoMini6E::new();
    
    // Test with zero input
    let zero_input = vec![0.0; 81];
    let (policy1, value1) = model.forward(&zero_input);
    
    // Test with non-zero input
    let nonzero_input: Vec<f32> = (0..81).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
    let (policy2, value2) = model.forward(&nonzero_input);
    
    // Policy outputs should be same (deterministic mock)
    assert_eq!(policy1, policy2);
    
    // Values should be different
    assert_ne!(value1, value2);
}
