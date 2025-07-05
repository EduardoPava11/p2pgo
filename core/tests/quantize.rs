// SPDX-License-Identifier: MIT OR Apache-2.0

//! Model quantization tests

/// Mock quantized model for testing
pub struct QuantizedGoMini6E {
    original_weights: Vec<f32>,
    quantized_weights: Vec<i8>,
    scale: f32,
}

impl QuantizedGoMini6E {
    pub fn from_trainer_model() -> Self {
        // Mock quantization: simulate int8 symmetric quantization without actual model
        let original_weights: Vec<f32> = (0..512).map(|i| (i as f32) / 512.0 - 0.5).collect();

        let max_val = original_weights
            .iter()
            .map(|&x| x.abs())
            .fold(0.0, f32::max);
        let scale = max_val / 127.0;

        let quantized_weights: Vec<i8> = original_weights
            .iter()
            .map(|&w| ((w / scale).round() as i8).clamp(-127, 127))
            .collect();

        Self {
            original_weights,
            quantized_weights,
            scale,
        }
    }

    pub fn forward(&self, input: &[f32]) -> (Vec<f32>, f32) {
        assert_eq!(input.len(), 81);

        // Dequantize weights and compute
        let dequantized: Vec<f32> = self
            .quantized_weights
            .iter()
            .map(|&w| w as f32 * self.scale)
            .collect();

        // Mock forward with quantized weights
        let policy_logits: Vec<f32> = (0..81)
            .map(|i| dequantized[i % dequantized.len()] + input[i] * 0.1)
            .collect();

        // Apply softmax normalization to policy
        let exp_logits: Vec<f32> = policy_logits.iter().map(|&x| x.exp()).collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        let policy: Vec<f32> = exp_logits.iter().map(|&x| x / sum_exp).collect();

        let value_scalar = (input.iter().sum::<f32>() / input.len() as f32).tanh(); // Keep value in [-1, 1]

        (policy, value_scalar)
    }
}

#[test]
fn test_model_quantization() {
    let quantized_model = QuantizedGoMini6E::from_trainer_model();

    // Check quantized weights are in valid range
    for &weight in &quantized_model.quantized_weights {
        assert!(weight >= -127 && weight <= 127);
    }

    // Check scale is positive
    assert!(quantized_model.scale > 0.0);
}

#[test]
fn test_quantized_inference() {
    let quantized_model = QuantizedGoMini6E::from_trainer_model();

    let input: Vec<f32> = (0..81).map(|i| (i as f32) / 81.0).collect();

    let (quant_policy, quant_value) = quantized_model.forward(&input);

    // Basic sanity checks on outputs
    assert_eq!(quant_policy.len(), 81); // Should match board size
    assert!(quant_value >= -1.0 && quant_value <= 1.0); // Game result range

    // Policy should sum to approximately 1.0 (softmax output)
    let policy_sum: f32 = quant_policy.iter().sum();
    assert!((policy_sum - 1.0).abs() < 0.1); // Allow some tolerance
}

#[test]
fn test_quantized_consistency() {
    let quantized_model = QuantizedGoMini6E::from_trainer_model();

    let input: Vec<f32> = (0..81).map(|i| if i == 40 { 1.0 } else { 0.0 }).collect();

    // Test that same input gives same output (deterministic)
    let (policy1, value1) = quantized_model.forward(&input);
    let (policy2, value2) = quantized_model.forward(&input);

    assert_eq!(policy1, policy2);
    assert_eq!(value1, value2);
}
