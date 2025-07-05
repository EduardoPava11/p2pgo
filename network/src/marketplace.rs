// SPDX-License-Identifier: MIT OR Apache-2.0

//! Marketplace for trading neural network weights that yield wins
//!
//! Since we lack supervised learning from expert games, we create
//! a decentralized marketplace where successful models can be traded.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::guilds::{Guild, BestPlayTracker};

/// A model listing in the marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListing {
    /// Unique identifier for this model
    pub model_id: String,

    /// Compressed model weights (CBOR encoded)
    pub weights_hash: [u8; 32],

    /// Size in bytes when compressed
    pub compressed_size: u64,

    /// Win rate statistics
    pub stats: ModelStats,

    /// Price in virtual credits or proof-of-play
    pub price: MarketPrice,

    /// Seller's relay node ID
    pub seller_id: String,

    /// Board positions this model excels at
    pub specialization: ModelSpecialization,

    /// Guild affinity of the model
    pub guild_affinity: Guild,

    /// Best play activation limits
    pub best_play_config: BestPlayTracker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    /// Total games played with this model
    pub games_played: u32,

    /// Win rate as percentage (0-100)
    pub win_rate: f32,

    /// Average game length in moves
    pub avg_game_length: f32,

    /// Win rate in different game phases
    pub phase_performance: PhaseStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseStats {
    /// Opening (moves 1-20) win rate
    pub opening: f32,

    /// Middle game (moves 21-60) advantage rate
    pub middle: f32,

    /// Endgame (moves 61+) conversion rate
    pub endgame: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketPrice {
    /// Free model (usually for bootstrapping)
    Free,

    /// Requires playing N games to unlock
    ProofOfPlay { games_required: u32 },

    /// Requires sharing your own training data
    DataExchange { min_games: u32, min_win_rate: f32 },

    /// Relay fuel credits (1 credit = 1 hop)
    FuelCredits { amount: u64 },

    /// DJED stablecoin payment
    Djed { amount: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelSpecialization {
    /// Good at all positions
    Generalist,

    /// Excels at opening theory
    OpeningSpecialist,

    /// Strong fighting/capturing style
    Fighter,

    /// Territory-oriented play
    Territorial,

    /// Endgame and counting specialist
    Endgame,

    /// Trained on specific opening patterns
    PatternSpecific { patterns: Vec<String> },
}

/// Monte Carlo confidence from hidden layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloHint {
    /// Which hidden layer had highest certainty
    pub most_confident_layer: usize,

    /// Confidence scores for top 3 moves
    pub top_moves: Vec<(String, f32)>,

    /// Uncertainty measure (0.0 = very certain, 1.0 = very uncertain)
    pub uncertainty: f32,

    /// Suggested computation budget for this position
    pub suggested_rollouts: u32,
}

/// Training contribution after each game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostGameTraining {
    /// Game ID that was just completed
    pub game_id: String,

    /// Self-play generated training samples
    pub training_samples: Vec<TrainingSample>,

    /// Model performance metrics
    pub performance: GamePerformance,

    /// Computational proof of training
    pub training_proof: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Board position (compressed)
    pub position: Vec<u8>,

    /// Policy target (move probabilities)
    pub policy: Vec<f32>,

    /// Value target (-1 to 1, from black's perspective)
    pub value: f32,

    /// Move number in game
    pub move_number: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePerformance {
    /// Did the model win this game?
    pub won: bool,

    /// Key mistakes identified
    pub mistakes: Vec<MoveEvaluation>,

    /// Brilliant moves found
    pub brilliancies: Vec<MoveEvaluation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveEvaluation {
    /// Move number
    pub move_num: u16,

    /// Actual move played
    pub played: String,

    /// Best move according to post-game analysis
    pub best: String,

    /// Score difference
    pub score_diff: f32,
}

/// Marketplace coordinator for a relay
pub struct MarketplaceCoordinator {
    /// Active model listings
    pub listings: HashMap<String, ModelListing>,

    /// Transaction history
    pub transactions: Vec<Transaction>,

    /// Reputation scores
    pub reputation: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub timestamp: u64,
    pub buyer_id: String,
    pub seller_id: String,
    pub model_id: String,
    pub price: MarketPrice,
    pub satisfaction_rating: Option<u8>, // 1-5 stars
}

impl MarketplaceCoordinator {
    pub fn new() -> Self {
        Self {
            listings: HashMap::new(),
            transactions: Vec::new(),
            reputation: HashMap::new(),
        }
    }

    /// List a new model on the marketplace
    pub fn list_model(&mut self, listing: ModelListing) {
        self.listings.insert(listing.model_id.clone(), listing);
    }

    /// Search for models by criteria
    pub fn search_models(&self, specialization: Option<ModelSpecialization>, min_win_rate: f32) -> Vec<&ModelListing> {
        self.listings.values()
            .filter(|listing| {
                listing.stats.win_rate >= min_win_rate &&
                specialization.as_ref().map_or(true, |spec| {
                    matches!(&listing.specialization, s if std::mem::discriminant(s) == std::mem::discriminant(spec))
                })
            })
            .collect()
    }

    /// Calculate reputation based on transaction history
    pub fn update_reputation(&mut self, seller_id: &str) {
        let seller_transactions: Vec<_> = self.transactions.iter()
            .filter(|t| t.seller_id == seller_id && t.satisfaction_rating.is_some())
            .collect();

        if !seller_transactions.is_empty() {
            let avg_rating = seller_transactions.iter()
                .map(|t| t.satisfaction_rating.unwrap() as f32)
                .sum::<f32>() / seller_transactions.len() as f32;

            self.reputation.insert(seller_id.to_string(), avg_rating);
        }
    }
}

/// Knowledge distillation for 9x9x9 games
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distilled9x9x9Model {
    /// Policy network for move selection (compressed to ~5MB)
    pub policy_net: CompressedNetwork,

    /// Value network for position evaluation (compressed to ~3MB)
    pub value_net: CompressedNetwork,

    /// Opening book for first 9 moves (~1MB)
    pub opening_book: HashMap<u64, Vec<(String, f32)>>,

    /// Endgame tablebase for last 9 moves (~1MB)
    pub endgame_db: HashMap<u64, f32>,

    /// Total size should be under 10MB
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedNetwork {
    /// Quantized weights (int8 instead of float32)
    pub weights: Vec<i8>,

    /// Scale factors for dequantization
    pub scales: Vec<f32>,

    /// Network architecture description
    pub architecture: String,

    /// Compression method used
    pub compression: CompressionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionMethod {
    /// Simple 8-bit quantization
    Int8Quantization,

    /// Pruning + quantization
    PrunedInt8 { sparsity: f32 },

    /// Knowledge distillation from larger model
    Distilled { teacher_size: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_search() {
        let mut coordinator = MarketplaceCoordinator::new();

        let listing = ModelListing {
            model_id: "test_model_1".to_string(),
            weights_hash: [0; 32],
            compressed_size: 8_000_000, // 8MB
            stats: ModelStats {
                games_played: 1000,
                win_rate: 65.5,
                avg_game_length: 85.0,
                phase_performance: PhaseStats {
                    opening: 68.0,
                    middle: 64.0,
                    endgame: 63.0,
                },
            },
            price: MarketPrice::ProofOfPlay { games_required: 10 },
            seller_id: "seller_123".to_string(),
            specialization: ModelSpecialization::Fighter,
        };

        coordinator.list_model(listing);

        let results = coordinator.search_models(Some(ModelSpecialization::Fighter), 60.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].model_id, "test_model_1");
    }
}