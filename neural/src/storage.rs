//! Neural network storage format

use serde::{Serialize, Deserialize};
use std::path::Path;
use anyhow::Result;
use std::collections::HashMap;

/// Neural network storage format
/// 
/// We use JSON for human readability and easy debugging.
/// For production, we could use:
/// - Binary formats (MessagePack, CBOR) for smaller size
/// - HDF5 for compatibility with Python ML tools
/// - ONNX for cross-platform inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralNetworkStorage {
    /// Format version for compatibility
    pub version: u32,
    
    /// Network metadata
    pub metadata: NetworkMetadata,
    
    /// Policy network weights
    pub policy_weights: PolicyWeights,
    
    /// Value network weights  
    pub value_weights: ValueWeights,
    
    /// Training history
    pub training_history: TrainingHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetadata {
    /// When the network was created
    pub created_at: String,
    
    /// Last training time
    pub last_trained: String,
    
    /// Configuration used
    pub config: crate::config::NeuralConfig,
    
    /// Network architecture info
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyWeights {
    /// Pattern-based weights for move prediction
    pub pattern_weights: HashMap<String, f32>,
    
    /// Opening book patterns
    pub opening_patterns: Vec<OpeningPattern>,
    
    /// Joseki sequences
    pub joseki_patterns: Vec<JosekiPattern>,
    
    /// Tactical patterns (ladders, nets, etc)
    pub tactical_patterns: Vec<TacticalPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueWeights {
    /// Territory evaluation weights
    pub territory_weights: TerritoryWeights,
    
    /// Influence evaluation weights
    pub influence_weights: InfluenceWeights,
    
    /// Group strength evaluation
    pub group_weights: GroupWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingHistory {
    /// Total games trained on
    pub total_games: usize,
    
    /// Games by source
    pub games_by_source: HashMap<String, usize>,
    
    /// Win rate during training
    pub training_win_rate: f32,
    
    /// Performance metrics
    pub metrics: Vec<TrainingMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningPattern {
    pub name: String,
    pub moves: Vec<(u8, u8)>,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JosekiPattern {
    pub corner: String,
    pub sequence: Vec<(u8, u8)>,
    pub evaluation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalPattern {
    pub pattern_type: String, // "ladder", "net", "snapback", etc
    pub shape: Vec<(i8, i8)>, // Relative coordinates
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerritoryWeights {
    pub corner_value: f32,
    pub side_value: f32,
    pub center_value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceWeights {
    pub wall_strength: f32,
    pub thickness_value: f32,
    pub moyo_potential: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupWeights {
    pub eye_space_value: f32,
    pub connection_strength: f32,
    pub escape_potential: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetric {
    pub epoch: usize,
    pub policy_accuracy: f32,
    pub value_accuracy: f32,
    pub loss: f32,
}

impl NeuralNetworkStorage {
    /// Current storage format version
    pub const CURRENT_VERSION: u32 = 1;
    
    /// Save to JSON file
    pub fn save_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Load from JSON file
    pub fn load_json(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let storage: Self = serde_json::from_str(&json)?;
        
        // Check version compatibility
        if storage.version > Self::CURRENT_VERSION {
            anyhow::bail!("Network file version {} is newer than supported version {}", 
                storage.version, Self::CURRENT_VERSION);
        }
        
        Ok(storage)
    }
    
    /// Export to compressed format
    pub fn save_compressed(&self, path: &Path) -> Result<()> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::fs::File;
        use std::io::Write;
        
        let file = File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        let json = serde_json::to_string(self)?;
        encoder.write_all(json.as_bytes())?;
        encoder.finish()?;
        Ok(())
    }
    
    /// Get a summary of the network
    pub fn summary(&self) -> String {
        format!(
            "Neural Network v{}\n\
             Created: {}\n\
             Trained on: {} games\n\
             Config: Aggression={}, Territory={}, Fighting={}\n\
             Patterns: {} opening, {} joseki, {} tactical",
            self.version,
            self.metadata.created_at,
            self.training_history.total_games,
            self.metadata.config.aggression,
            self.metadata.config.territory_focus,
            self.metadata.config.fighting_spirit,
            self.policy_weights.opening_patterns.len(),
            self.policy_weights.joseki_patterns.len(),
            self.policy_weights.tactical_patterns.len(),
        )
    }
}