use serde::{Deserialize, Serialize};

/// RNA (Training Data) Message Types
/// Inspired by biological RNA:
/// - mRNA: Full game data (messenger RNA carries complete information)
/// - tRNA: Pattern data (transfer RNA carries specific patterns)
/// - miRNA: Regulatory signals (micro RNA regulates expression)
/// - lncRNA: Style transfer data (long non-coding RNA)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RNAMessage {
    /// Unique message ID
    pub id: String,
    /// Source peer ID
    pub source_peer: String,
    /// RNA type
    pub rna_type: RNAType,
    /// Timestamp
    pub timestamp: u64,
    /// Quality score (0.0 to 1.0)
    pub quality_score: f32,
    /// Additional data payload
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RNAType {
    /// Full game data (mRNA)
    SGFData {
        sgf_content: String,
        move_range: (usize, usize),
        player_ranks: (String, String),
    },
    
    /// Pattern data (tRNA) 
    PatternData {
        pattern_type: String,
        board_region: (usize, usize, usize, usize), // x, y, width, height
        frequency: f32,
    },
    
    /// Model weight updates
    ModelWeights {
        model_type: String,
        layer_updates: Vec<Vec<f32>>,
        consensus_count: usize,
    },
    
    /// Regulatory signal (miRNA)
    RegulatorySignal {
        signal_type: String,
        value: f32,
        confidence: f32,
    },
    
    /// Style transfer data (lncRNA)
    StyleTransfer {
        style_name: String,
        style_vector: Vec<f32>,
        source_player: String,
    },
    
    /// Relay discovery
    RelayDiscovery {
        addresses: Vec<String>,
        discovery_score: f32,
        capabilities: Vec<String>,
    },
    
    /// Training consensus
    TrainingConsensus {
        epoch: u32,
        participants: Vec<String>,
        consensus_weights: Vec<f32>,
        agreement_score: f32,
    },
}