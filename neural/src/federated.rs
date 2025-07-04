//! Federated Learning implementation for 9x9 Go micro-nets

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compressed weight delta for gossip protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightDelta {
    /// Model revision this delta is based on
    pub base_revision: u32,
    /// Target revision after applying delta
    pub target_revision: u32,
    /// Compressed weight updates (layer_name -> weight_delta)
    pub deltas: HashMap<String, Vec<i8>>, // 8-bit quantized
    /// Number of training steps in this delta
    pub train_steps: u32,
    /// Device ID that created this delta
    pub device_id: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Federated learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    /// How often to publish deltas (in seconds)
    pub publish_interval: u32,
    /// Minimum training steps before publishing
    pub min_train_steps: u32,
    /// Maximum delta size in bytes
    pub max_delta_size: usize,
    /// Gradient clipping threshold
    pub gradient_clip: f32,
    /// Quantization bits (8 for mobile)
    pub quantization_bits: u8,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            publish_interval: 600, // 10 minutes
            min_train_steps: 100,
            max_delta_size: 30_000, // 30KB
            gradient_clip: 1.0,
            quantization_bits: 8,
        }
    }
}

/// Federated learning coordinator
pub struct FederatedLearning {
    config: FederatedConfig,
    current_revision: u32,
    accumulated_steps: u32,
    weight_buffer: HashMap<String, Vec<f32>>,
}

impl FederatedLearning {
    pub fn new(config: FederatedConfig) -> Self {
        Self {
            config,
            current_revision: 0,
            accumulated_steps: 0,
            weight_buffer: HashMap::new(),
        }
    }
    
    /// Accumulate gradients from local training
    pub fn accumulate_gradients(&mut self, gradients: HashMap<String, Vec<f32>>) {
        for (layer, grads) in gradients {
            let buffer = self.weight_buffer.entry(layer).or_insert_with(Vec::new);
            if buffer.is_empty() {
                *buffer = grads;
            } else {
                // Add gradients element-wise
                for (i, grad) in grads.iter().enumerate() {
                    if i < buffer.len() {
                        buffer[i] += grad;
                    }
                }
            }
        }
        self.accumulated_steps += 1;
    }
    
    /// Create weight delta for gossip
    pub fn create_delta(&mut self, device_id: String) -> Option<WeightDelta> {
        if self.accumulated_steps < self.config.min_train_steps {
            return None;
        }
        
        let mut deltas = HashMap::new();
        let mut total_size = 0;
        
        for (layer, weights) in &self.weight_buffer {
            // Average accumulated gradients
            let avg_weights: Vec<f32> = weights.iter()
                .map(|w| w / self.accumulated_steps as f32)
                .collect();
            
            // Clip gradients
            let clipped: Vec<f32> = avg_weights.iter()
                .map(|w| w.max(-self.config.gradient_clip).min(self.config.gradient_clip))
                .collect();
            
            // Quantize to 8-bit
            let quantized = Self::quantize_weights(&clipped, self.config.quantization_bits);
            
            total_size += quantized.len();
            if total_size > self.config.max_delta_size {
                break; // Don't exceed size limit
            }
            
            deltas.insert(layer.clone(), quantized);
        }
        
        let delta = WeightDelta {
            base_revision: self.current_revision,
            target_revision: self.current_revision + 1,
            deltas,
            train_steps: self.accumulated_steps,
            device_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        // Clear buffers
        self.weight_buffer.clear();
        self.accumulated_steps = 0;
        self.current_revision += 1;
        
        Some(delta)
    }
    
    /// Quantize weights to reduced precision
    fn quantize_weights(weights: &[f32], bits: u8) -> Vec<i8> {
        let scale = (1 << (bits - 1)) as f32 - 1.0;
        weights.iter()
            .map(|w| (w * scale).round().max(-128.0).min(127.0) as i8)
            .collect()
    }
    
    /// Dequantize weights back to f32
    pub fn dequantize_weights(quantized: &[i8], bits: u8) -> Vec<f32> {
        let scale = (1 << (bits - 1)) as f32 - 1.0;
        quantized.iter()
            .map(|&q| q as f32 / scale)
            .collect()
    }
}

/// SuperNode aggregator for FedAvg
pub struct FederatedAggregator {
    collected_deltas: Vec<WeightDelta>,
    base_revision: u32,
}

impl FederatedAggregator {
    pub fn new(base_revision: u32) -> Self {
        Self {
            collected_deltas: Vec::new(),
            base_revision,
        }
    }
    
    /// Add a delta from a peer
    pub fn add_delta(&mut self, delta: WeightDelta) -> Result<(), String> {
        if delta.base_revision != self.base_revision {
            return Err(format!(
                "Delta base revision {} doesn't match aggregator base {}",
                delta.base_revision, self.base_revision
            ));
        }
        
        self.collected_deltas.push(delta);
        Ok(())
    }
    
    /// Perform FedAvg aggregation
    pub fn aggregate(&self) -> Option<HashMap<String, Vec<f32>>> {
        if self.collected_deltas.is_empty() {
            return None;
        }
        
        let mut aggregated: HashMap<String, Vec<f32>> = HashMap::new();
        let mut layer_counts: HashMap<String, usize> = HashMap::new();
        
        // Sum all deltas
        for delta in &self.collected_deltas {
            for (layer, quantized) in &delta.deltas {
                let dequantized = FederatedLearning::dequantize_weights(quantized, 8);
                
                let sum = aggregated.entry(layer.clone()).or_insert_with(Vec::new);
                if sum.is_empty() {
                    *sum = dequantized;
                } else {
                    for (i, val) in dequantized.iter().enumerate() {
                        if i < sum.len() {
                            sum[i] += val;
                        }
                    }
                }
                
                *layer_counts.entry(layer.clone()).or_insert(0) += 1;
            }
        }
        
        // Average the sums
        for (layer, sum) in aggregated.iter_mut() {
            if let Some(&count) = layer_counts.get(layer) {
                for val in sum.iter_mut() {
                    *val /= count as f32;
                }
            }
        }
        
        Some(aggregated)
    }
    
    /// Get number of collected deltas
    pub fn delta_count(&self) -> usize {
        self.collected_deltas.len()
    }
}

/// Knowledge distillation for model compression
pub struct KnowledgeDistillation {
    temperature: f32,
    alpha: f32, // Weight for teacher loss
}

impl KnowledgeDistillation {
    pub fn new() -> Self {
        Self {
            temperature: 3.0, // Soften probabilities
            alpha: 0.7,       // 70% teacher, 30% hard labels
        }
    }
    
    /// Calculate distillation loss
    pub fn distillation_loss(
        &self,
        student_logits: &[f32],
        teacher_logits: &[f32],
        true_labels: &[u8],
    ) -> f32 {
        // Soft targets from teacher
        let teacher_probs = self.softmax_with_temperature(teacher_logits);
        let student_probs = self.softmax_with_temperature(student_logits);
        
        // KL divergence for soft targets
        let soft_loss: f32 = teacher_probs.iter()
            .zip(student_probs.iter())
            .map(|(t, s)| t * (t / s).ln())
            .sum();
        
        // Cross-entropy for hard labels
        let hard_loss = self.cross_entropy(student_logits, true_labels);
        
        // Combined loss
        self.alpha * soft_loss * self.temperature.powi(2) + (1.0 - self.alpha) * hard_loss
    }
    
    fn softmax_with_temperature(&self, logits: &[f32]) -> Vec<f32> {
        let scaled: Vec<f32> = logits.iter().map(|x| x / self.temperature).collect();
        let max = scaled.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_sum: f32 = scaled.iter().map(|x| (x - max).exp()).sum();
        scaled.iter().map(|x| (x - max).exp() / exp_sum).collect()
    }
    
    fn cross_entropy(&self, logits: &[f32], labels: &[u8]) -> f32 {
        let probs = self.softmax_with_temperature(logits);
        labels.iter()
            .enumerate()
            .map(|(i, &label)| {
                if label == 1 {
                    -probs[i].ln()
                } else {
                    0.0
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{FederatedLearning, FederatedAggregator, WeightDelta};
    use std::collections::HashMap;
    
    #[test]
    fn test_weight_quantization() {
        let weights = vec![0.5, -0.3, 0.8, -0.9, 0.1];
        let quantized = FederatedLearning::quantize_weights(&weights, 8);
        let dequantized = FederatedLearning::dequantize_weights(&quantized, 8);
        
        // Check approximate equality
        for (orig, deq) in weights.iter().zip(dequantized.iter()) {
            assert!((orig - deq).abs() < 0.01);
        }
    }
    
    #[test]
    fn test_federated_aggregation() {
        let mut aggregator = FederatedAggregator::new(0);
        
        // Add some mock deltas
        for i in 0..3 {
            let mut deltas = HashMap::new();
            deltas.insert("layer1".to_string(), vec![i as i8; 10]);
            
            let delta = WeightDelta {
                base_revision: 0,
                target_revision: 1,
                deltas,
                train_steps: 100,
                device_id: format!("device_{}", i),
                timestamp: 0,
            };
            
            aggregator.add_delta(delta).unwrap();
        }
        
        let result = aggregator.aggregate().unwrap();
        assert!(result.contains_key("layer1"));
        
        // Should average to approximately 1.0
        let avg = &result["layer1"];
        assert!((avg[0] - 0.0078).abs() < 0.001); // (0+1+2)/3 / 127
    }
}