# Simple Training Data Marketplace

## Core Idea

Instead of complex tokenomics, we just need:
1. A way to track which training data is good
2. A way to share neural network improvements between relays
3. A simple reputation system for players and relays

## Federated Learning Architecture

### 1. Local Training at Each Relay

```rust
// relay/src/federated_trainer.rs
pub struct FederatedTrainer {
    /// Current local model
    local_model: GoNeuralNet,
    /// Training data from games at this relay
    local_games: Vec<GameRecord>,
    /// Model version/generation
    generation: u32,
}

impl FederatedTrainer {
    /// Train on local games every N games
    pub async fn train_local_batch(&mut self) -> ModelUpdate {
        // Train only on games with consensus
        let quality_games: Vec<_> = self.local_games.iter()
            .filter(|g| g.consensus_achieved)
            .collect();
        
        // Simple SGD on local data
        let gradients = self.local_model.compute_gradients(&quality_games);
        
        ModelUpdate {
            relay_id: self.relay_id,
            generation: self.generation,
            gradients,
            games_trained: quality_games.len(),
            avg_consensus_rate: calculate_avg_consensus(&quality_games),
        }
    }
}
```

### 2. Model Aggregation Protocol

```rust
// Simple federated averaging
pub struct ModelAggregator {
    /// Pending updates from relays
    pending_updates: Vec<ModelUpdate>,
    /// Minimum updates before aggregation
    min_updates: usize,
}

impl ModelAggregator {
    /// Aggregate when we have enough updates
    pub fn try_aggregate(&mut self) -> Option<GlobalModel> {
        if self.pending_updates.len() < self.min_updates {
            return None;
        }
        
        // Weight by number of quality games
        let total_games: usize = self.pending_updates.iter()
            .map(|u| u.games_trained)
            .sum();
        
        // Federated averaging
        let mut averaged_weights = vec![0.0; MODEL_SIZE];
        
        for update in &self.pending_updates {
            let weight = update.games_trained as f32 / total_games as f32;
            for (i, grad) in update.gradients.iter().enumerate() {
                averaged_weights[i] += grad * weight;
            }
        }
        
        Some(GlobalModel {
            generation: self.current_generation + 1,
            weights: averaged_weights,
            contributors: self.pending_updates.iter()
                .map(|u| u.relay_id.clone())
                .collect(),
        })
    }
}
```

### 3. Simple P2P Model Sharing

```rust
// network/src/model_gossip.rs
pub enum ModelMessage {
    /// Share local model update
    LocalUpdate {
        update: ModelUpdate,
        proof_of_training: Hash, // Hash of training games
    },
    
    /// Request better model
    ModelRequest {
        current_generation: u32,
        current_performance: f32,
    },
    
    /// Share aggregated model
    GlobalModel {
        model: GlobalModel,
        performance_metrics: PerformanceMetrics,
    },
}

impl RelayNode {
    /// Gossip model updates to peers
    pub async fn share_model_update(&self, update: ModelUpdate) {
        // Only share with peers that have good training data
        let quality_peers = self.peers.iter()
            .filter(|p| p.reputation > 0.8)
            .take(5);
        
        for peer in quality_peers {
            peer.send(ModelMessage::LocalUpdate {
                update: update.clone(),
                proof_of_training: blake3::hash(&update.games_trained),
            }).await;
        }
    }
}
```

## Simple Marketplace Contract

Just track reputation and model performance:

```rust
#[ink::contract]
mod training_marketplace {
    use ink_storage::Mapping;
    
    #[ink(storage)]
    pub struct TrainingMarketplace {
        /// Player reputation (0-100)
        player_reputation: Mapping<AccountId, u8>,
        /// Model performance by hash
        model_performance: Mapping<Hash, ModelMetrics>,
        /// Best model for each generation
        best_models: Mapping<u32, Hash>,
    }
    
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    pub struct ModelMetrics {
        pub win_rate: u32,          // Against previous generation
        pub consensus_rate: u32,    // How often consensus achieved
        pub contributors: Vec<AccountId>,
        pub games_played: u32,
    }
    
    impl TrainingMarketplace {
        /// Record game result to update reputation
        #[ink(message)]
        pub fn record_game(&mut self, 
            black: AccountId,
            white: AccountId,
            consensus_achieved: bool,
            winner: Option<AccountId>,
        ) {
            // Simple reputation: +1 for consensus, -1 for no consensus
            if consensus_achieved {
                self.adjust_reputation(black, 1);
                self.adjust_reputation(white, 1);
            } else {
                self.adjust_reputation(black, -1);
                self.adjust_reputation(white, -1);
            }
        }
        
        /// Submit model performance metrics
        #[ink(message)]
        pub fn submit_model(&mut self,
            model_hash: Hash,
            generation: u32,
            metrics: ModelMetrics,
        ) {
            self.model_performance.insert(&model_hash, &metrics);
            
            // Update best model if better
            if let Some(current_best) = self.best_models.get(&generation) {
                let current_metrics = self.model_performance.get(&current_best).unwrap();
                if metrics.win_rate > current_metrics.win_rate {
                    self.best_models.insert(&generation, &model_hash);
                }
            } else {
                self.best_models.insert(&generation, &model_hash);
            }
        }
        
        /// Get training data contributors for rewards
        #[ink(message)]
        pub fn get_contributors(&self, model_hash: Hash) -> Vec<AccountId> {
            self.model_performance.get(&model_hash)
                .map(|m| m.contributors)
                .unwrap_or_default()
        }
    }
}
```

## Relay Configuration for Federated Learning

### 1. Simple Relay Config

```toml
# relay_config.toml
[federation]
# Train every N games
local_batch_size = 100

# Share updates every N minutes
update_interval = 300

# Minimum peers for aggregation
min_aggregation_peers = 3

# Only accept data from players with reputation > X
min_player_reputation = 50

# Model storage
model_cache_size = 10
model_directory = "./models"

[training]
# Simple hyperparameters
learning_rate = 0.001
batch_size = 32
validation_split = 0.2

# Focus on consensus games
require_consensus = true
min_game_length = 20
```

### 2. Relay Federation Protocol

```rust
// Simple relay coordination
pub struct RelayFederation {
    /// Our training schedule
    schedule: TrainingSchedule,
    /// Current model performance
    current_performance: f32,
    /// Peer performances
    peer_performances: HashMap<String, f32>,
}

impl RelayFederation {
    /// Decide when to train based on peer activity
    pub fn should_train_now(&self) -> bool {
        // Train if we have enough new games
        if self.new_games_since_training() >= 100 {
            return true;
        }
        
        // Train if peers have much better models
        let best_peer = self.peer_performances.values().max();
        if let Some(best) = best_peer {
            if best - self.current_performance > 0.1 {
                return true;
            }
        }
        
        false
    }
    
    /// Simple peer selection for federation
    pub fn select_federation_peers(&self) -> Vec<String> {
        // Choose peers with similar game volume
        let our_volume = self.games_per_day();
        
        self.peer_performances.keys()
            .filter(|peer| {
                let peer_volume = self.get_peer_volume(peer);
                (peer_volume - our_volume).abs() < 100
            })
            .take(5)
            .cloned()
            .collect()
    }
}
```

## Training Data Quality Ranking

Simple quality metrics without complex economics:

```rust
pub struct TrainingDataQuality {
    /// Game-level quality
    pub consensus_achieved: bool,
    pub move_validity_rate: f32,
    pub game_length: u32,
    pub player_reputations: (u8, u8),
    
    /// Dataset-level quality
    pub total_games: u32,
    pub consensus_rate: f32,
    pub avg_game_length: f32,
    pub unique_players: u32,
}

impl TrainingDataQuality {
    /// Simple quality score (0-100)
    pub fn quality_score(&self) -> u8 {
        let mut score = 0u8;
        
        // Consensus is most important
        if self.consensus_achieved {
            score += 40;
        }
        
        // Game length (cap at 200 moves)
        score += (self.game_length.min(200) / 4) as u8; // 0-50 points
        
        // Player reputation
        let avg_reputation = (self.player_reputations.0 + self.player_reputations.1) / 2;
        score += avg_reputation / 10; // 0-10 points
        
        score.min(100)
    }
}
```

## Implementation Priority

1. **Phase 1: Local Training (Week 1)**
   - Each relay trains on its own data
   - Simple neural net for move prediction
   - Track win rates locally

2. **Phase 2: Model Sharing (Week 2)**
   - Gossip model updates between relays
   - Simple averaging of weights
   - Test with 3-5 relays

3. **Phase 3: Quality Tracking (Week 3)**
   - Deploy simple contract for reputation
   - Track which models perform best
   - Basic leaderboard

4. **Phase 4: Federation (Week 4)**
   - Coordinate training schedules
   - Weighted aggregation by quality
   - Performance benchmarks

## Why This Works

1. **No Complex Economics**: Just track what works
2. **Natural Selection**: Bad models naturally get replaced
3. **Simple Reputation**: Players who achieve consensus get higher reputation
4. **Federated Learning**: Each relay contributes without sharing raw data
5. **Quality Focus**: Only train on games with consensus

## Next Steps

1. Implement basic neural net in Rust
2. Add training loop to relay code
3. Test model sharing between 2 relays
4. Deploy simple reputation contract
5. Benchmark model improvements

The key insight: We don't need complex markets. We just need to:
- Know which training data is good (consensus games)
- Share model improvements between relays
- Track simple reputation to filter bad actors

Everything else emerges naturally from relays wanting better models to attract players.