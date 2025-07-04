# Relay Profitability & Training Data Ownership Spec

## Core Concept: Training Data as Shared Asset

When two players complete a game with consensus, both players' relays get a "stamp" to use that training data. Non-consensus games are immediately discarded.

## 1. Simplified ink! Contract with Data Stamps

```rust
#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod training_marketplace {
    use ink_storage::{traits::SpreadAllocate, Mapping};
    use ink_prelude::vec::Vec;
    
    /// Training data stamp - proof that a relay can use this data
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TrainingStamp {
        pub game_hash: Hash,
        pub black_relay: AccountId,
        pub white_relay: AccountId,
        pub consensus_move: u32,  // Move number where consensus achieved
        pub total_moves: u32,
        pub game_features: GameFeatures,
        pub timestamp: Timestamp,
    }
    
    /// Extracted game features for training
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GameFeatures {
        pub opening_pattern: u8,      // 0-255 opening classification
        pub fighting_intensity: u8,   // 0-100 how many captures
        pub territory_balance: i8,    // -100 to +100 (black vs white)
        pub game_phase_transitions: [u32; 3], // moves where game phase changed
        pub complexity_score: u8,     // 0-100 based on branching factor
    }
    
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct TrainingMarketplace {
        /// Training stamps by game hash
        training_stamps: Mapping<Hash, TrainingStamp>,
        /// Which relays can use which training data
        relay_access: Mapping<(AccountId, Hash), bool>,
        /// Relay reputation (0-100)
        relay_reputation: Mapping<AccountId, u8>,
        /// Model registry
        models: Mapping<Hash, ModelRecord>,
        /// Gas subsidy pool for profitable relays
        subsidy_pool: Balance,
        /// Minimum consensus rate for stamp creation
        min_consensus_rate: u8,
    }
    
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ModelRecord {
        pub generation: u32,
        pub training_stamps_used: Vec<Hash>,
        pub performance: ModelPerformance,
        pub contributors: Vec<AccountId>,
    }
    
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ModelPerformance {
        pub win_rate: u8,           // 0-100
        pub prediction_accuracy: u8, // 0-100 for next move
        pub style_consistency: u8,   // 0-100 how well it maintains style
    }
    
    /// Events
    #[ink(event)]
    pub struct TrainingStampCreated {
        #[ink(topic)]
        game_hash: Hash,
        black_relay: AccountId,
        white_relay: AccountId,
        features: GameFeatures,
    }
    
    #[ink(event)]
    pub struct ModelSubmitted {
        #[ink(topic)]
        model_hash: Hash,
        generation: u32,
        stamps_used: u32,
        performance: ModelPerformance,
    }
    
    #[ink(event)]
    pub struct SubsidyPaid {
        #[ink(topic)]
        relay: AccountId,
        amount: Balance,
        reason: SubsidyReason,
    }
    
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum SubsidyReason {
        HighQualityData,
        ModelImprovement,
        ConsistentParticipation,
    }
    
    impl TrainingMarketplace {
        #[ink(constructor)]
        pub fn new(min_consensus_rate: u8) -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                contract.min_consensus_rate = min_consensus_rate;
                contract.subsidy_pool = 0;
            })
        }
        
        /// Batch submit game results - more gas efficient
        #[ink(message)]
        pub fn batch_submit_games(
            &mut self,
            games: Vec<(Hash, AccountId, AccountId, bool, GameFeatures)>
        ) -> Result<u32, Error> {
            let mut stamps_created = 0u32;
            
            for (game_hash, black_relay, white_relay, consensus, features) in games {
                if consensus {
                    // Create training stamp
                    let stamp = TrainingStamp {
                        game_hash,
                        black_relay,
                        white_relay,
                        consensus_move: features.game_phase_transitions[1], // Middle game
                        total_moves: features.game_phase_transitions[2],    // End
                        game_features: features.clone(),
                        timestamp: self.env().block_timestamp(),
                    };
                    
                    // Store stamp
                    self.training_stamps.insert(&game_hash, &stamp);
                    
                    // Grant access to both relays
                    self.relay_access.insert(&(black_relay, game_hash), &true);
                    self.relay_access.insert(&(white_relay, game_hash), &true);
                    
                    // Update reputation
                    self.increase_reputation(black_relay);
                    self.increase_reputation(white_relay);
                    
                    self.env().emit_event(TrainingStampCreated {
                        game_hash,
                        black_relay,
                        white_relay,
                        features,
                    });
                    
                    stamps_created += 1;
                }
                // Non-consensus games are simply not recorded (discarded)
            }
            
            // Pay gas subsidy if enough quality stamps
            if stamps_created >= 10 {
                self.pay_subsidy(self.env().caller(), SubsidyReason::HighQualityData);
            }
            
            Ok(stamps_created)
        }
        
        /// Submit trained model with proof of training stamps used
        #[ink(message)]
        pub fn submit_model(
            &mut self,
            model_hash: Hash,
            generation: u32,
            stamp_hashes: Vec<Hash>,
            performance: ModelPerformance,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Verify relay has access to all claimed stamps
            let mut valid_stamps = Vec::new();
            for stamp_hash in stamp_hashes {
                if self.relay_access.get(&(caller, stamp_hash)).unwrap_or(false) {
                    valid_stamps.push(stamp_hash);
                }
            }
            
            if valid_stamps.is_empty() {
                return Err(Error::NoValidTrainingData);
            }
            
            // Get all contributors
            let mut contributors = vec![caller];
            for stamp_hash in &valid_stamps {
                if let Some(stamp) = self.training_stamps.get(&stamp_hash) {
                    if !contributors.contains(&stamp.black_relay) {
                        contributors.push(stamp.black_relay);
                    }
                    if !contributors.contains(&stamp.white_relay) {
                        contributors.push(stamp.white_relay);
                    }
                }
            }
            
            let model = ModelRecord {
                generation,
                training_stamps_used: valid_stamps,
                performance,
                contributors,
            };
            
            self.models.insert(&model_hash, &model);
            
            // Pay subsidy for good models
            if performance.win_rate > 60 && performance.prediction_accuracy > 70 {
                self.pay_subsidy(caller, SubsidyReason::ModelImprovement);
            }
            
            self.env().emit_event(ModelSubmitted {
                model_hash,
                generation,
                stamps_used: model.training_stamps_used.len() as u32,
                performance,
            });
            
            Ok(())
        }
        
        /// Burn training stamps to recover storage deposit
        #[ink(message)]
        pub fn burn_stamps(&mut self, stamp_hashes: Vec<Hash>) -> Result<Balance, Error> {
            let caller = self.env().caller();
            let mut burned = 0u32;
            let mut recovered_deposit = 0u128;
            
            for stamp_hash in stamp_hashes {
                // Check access
                if !self.relay_access.get(&(caller, stamp_hash)).unwrap_or(false) {
                    continue;
                }
                
                // Remove stamp
                if let Some(stamp) = self.training_stamps.get(&stamp_hash) {
                    // Only allow burning if stamp is old (>30 days)
                    let age = self.env().block_timestamp() - stamp.timestamp;
                    if age > 30 * 24 * 60 * 60 * 1000 { // 30 days in ms
                        self.training_stamps.remove(&stamp_hash);
                        self.relay_access.remove(&(stamp.black_relay, stamp_hash));
                        self.relay_access.remove(&(stamp.white_relay, stamp_hash));
                        
                        burned += 1;
                        recovered_deposit += 1_000_000; // Fixed deposit per stamp
                    }
                }
            }
            
            if burned > 0 {
                // Return deposit to caller
                self.env().transfer(caller, recovered_deposit).map_err(|_| Error::TransferFailed)?;
            }
            
            Ok(recovered_deposit)
        }
        
        /// Fund the subsidy pool
        #[ink(message, payable)]
        pub fn fund_subsidy_pool(&mut self) {
            self.subsidy_pool += self.env().transferred_value();
        }
        
        /// Internal: Pay subsidy to relay
        fn pay_subsidy(&mut self, relay: AccountId, reason: SubsidyReason) {
            let amount = match reason {
                SubsidyReason::HighQualityData => 100_000_000,      // 0.1 token
                SubsidyReason::ModelImprovement => 500_000_000,     // 0.5 token
                SubsidyReason::ConsistentParticipation => 50_000_000, // 0.05 token
            };
            
            if self.subsidy_pool >= amount {
                self.subsidy_pool -= amount;
                let _ = self.env().transfer(relay, amount);
                
                self.env().emit_event(SubsidyPaid {
                    relay,
                    amount,
                    reason,
                });
            }
        }
        
        fn increase_reputation(&mut self, relay: AccountId) {
            let current = self.relay_reputation.get(&relay).unwrap_or(50);
            self.relay_reputation.insert(&relay, &(current + 1).min(100));
        }
    }
}
```

## 2. Game Feature Extraction Tools

Go players need tools to analyze their games beyond simple win/loss:

```rust
// core/src/game_analysis.rs
pub struct GameAnalyzer {
    /// Pattern library for opening classification
    opening_patterns: Vec<OpeningPattern>,
    /// Joseki database
    joseki_db: JosekiDatabase,
}

pub struct OpeningPattern {
    pub name: String,
    pub moves: Vec<Coord>,
    pub style: PlayStyle,
}

#[derive(Clone, Copy)]
pub enum PlayStyle {
    Territorial,    // Focus on corners and sides
    Influential,    // Focus on center and moyos
    Fighting,       // Many captures and ko fights
    Balanced,       // Mix of approaches
}

impl GameAnalyzer {
    /// Extract features from a completed game
    pub fn extract_features(&self, game: &GameState) -> GameFeatures {
        GameFeatures {
            opening_pattern: self.classify_opening(&game.moves[..20.min(game.moves.len())]),
            fighting_intensity: self.calculate_fighting_intensity(game),
            territory_balance: self.calculate_territory_balance(game),
            game_phase_transitions: self.detect_phase_transitions(game),
            complexity_score: self.calculate_complexity(game),
        }
    }
    
    /// Classify opening pattern (0-255)
    fn classify_opening(&self, moves: &[Move]) -> u8 {
        // Match against known patterns
        for (i, pattern) in self.opening_patterns.iter().enumerate() {
            if self.matches_pattern(moves, pattern) {
                return i as u8;
            }
        }
        255 // Unknown pattern
    }
    
    /// Calculate how much fighting occurred (0-100)
    fn calculate_fighting_intensity(&self, game: &GameState) -> u8 {
        let total_captures = game.black_captures + game.white_captures;
        let ko_fights = game.moves.windows(4).filter(|w| self.is_ko_fight(w)).count();
        
        // Normalize to 0-100
        ((total_captures * 5 + ko_fights * 10).min(100)) as u8
    }
    
    /// Detect game phase transitions
    fn detect_phase_transitions(&self, game: &GameState) -> [u32; 3] {
        let mut transitions = [0u32; 3];
        
        // Opening -> Middle game (usually around move 30-50)
        for (i, window) in game.moves.windows(10).enumerate() {
            if self.is_middle_game_pattern(window) {
                transitions[0] = i as u32;
                break;
            }
        }
        
        // Middle -> End game (territory solidifies)
        for (i, window) in game.moves[transitions[0] as usize..].windows(10).enumerate() {
            if self.is_endgame_pattern(window) {
                transitions[1] = (transitions[0] + i as u32);
                break;
            }
        }
        
        // Game end
        transitions[2] = game.moves.len() as u32;
        
        transitions
    }
}

/// Advanced analysis tools
pub struct AdvancedTools {
    /// Heat map of move frequency
    pub move_heatmap: [[u32; 19]; 19],
    /// Influence map at each move
    pub influence_progression: Vec<InfluenceMap>,
    /// Shape recognition
    pub shapes_formed: Vec<Shape>,
}

pub struct Shape {
    pub shape_type: ShapeType,
    pub coords: Vec<Coord>,
    pub move_number: u32,
    pub stability: f32, // How likely to survive
}

pub enum ShapeType {
    Eye,
    FalseEye,
    Tiger,
    Bamboo,
    Net,
    Ladder,
    Ko,
    Seki,
}
```

## 3. Relay Profitability Implementation

```rust
// relay/src/profitability.rs
pub struct ProfitabilityTracker {
    /// Gas costs per operation
    gas_costs: GasCosts,
    /// Subsidy earned
    subsidies_earned: u128,
    /// Current batch
    current_batch: GameBatch,
}

pub struct GasCosts {
    pub batch_submit: u128,    // ~50,000 gas
    pub model_submit: u128,    // ~100,000 gas
    pub burn_stamps: u128,     // ~30,000 gas
    pub current_gas_price: u128,
}

pub struct GameBatch {
    pub games: Vec<(Hash, AccountId, AccountId, bool, GameFeatures)>,
    pub start_time: Instant,
}

impl ProfitabilityTracker {
    /// Decide when to submit batch
    pub fn should_submit_batch(&self) -> bool {
        // Calculate potential subsidy
        let consensus_games = self.current_batch.games.iter()
            .filter(|(_, _, _, consensus, _)| *consensus)
            .count();
        
        if consensus_games >= 10 {
            // Guaranteed subsidy for high quality data
            return true;
        }
        
        // Check if gas price is low
        if self.gas_costs.current_gas_price < self.gas_costs.batch_submit / 100 {
            return consensus_games >= 5;
        }
        
        // Time-based trigger
        self.current_batch.start_time.elapsed() > Duration::from_secs(3600)
    }
    
    /// Estimate profit for current batch
    pub fn estimate_batch_profit(&self) -> i128 {
        let consensus_count = self.current_batch.games.iter()
            .filter(|(_, _, _, consensus, _)| *consensus)
            .count() as u128;
        
        let subsidy = if consensus_count >= 10 {
            100_000_000 // HighQualityData subsidy
        } else {
            0
        };
        
        let gas_cost = self.gas_costs.batch_submit * self.gas_costs.current_gas_price;
        
        (subsidy as i128) - (gas_cost as i128)
    }
}

/// Optimal batching strategy
pub struct BatchOptimizer {
    /// Historical gas prices
    gas_price_history: VecDeque<(Timestamp, u128)>,
    /// Predicted gas prices
    gas_predictor: GasPredictor,
}

impl BatchOptimizer {
    pub fn optimal_batch_size(&self, current_games: usize) -> usize {
        // If we have enough for subsidy, submit now
        if current_games >= 10 {
            return current_games;
        }
        
        // Predict if gas will be cheaper later
        let current_price = self.gas_price_history.back().map(|(_, p)| *p).unwrap_or(1);
        let predicted_price = self.gas_predictor.predict_next_hour();
        
        if predicted_price < current_price * 0.8 {
            // Wait for cheaper gas
            0
        } else {
            // Submit if we have at least 5
            current_games.max(5)
        }
    }
}
```

## 4. Tool Integration for Players

```rust
// ui/src/analysis_tools.rs
pub struct PlayerTools {
    /// Post-game analysis
    pub game_reviewer: GameReviewer,
    /// Style classifier
    pub style_analyzer: StyleAnalyzer,
    /// Mistake finder
    pub mistake_detector: MistakeDetector,
}

pub struct GameReviewer {
    /// AI model for move evaluation
    evaluator: MoveEvaluator,
}

impl GameReviewer {
    /// Generate review of completed game
    pub fn review_game(&self, game: &GameState) -> GameReview {
        let mut critical_moves = Vec::new();
        let mut blunders = Vec::new();
        
        for (i, move_) in game.moves.iter().enumerate() {
            let evaluation = self.evaluator.evaluate_position(&game, i);
            
            if evaluation.swing.abs() > 10.0 {
                critical_moves.push(CriticalMove {
                    move_number: i as u32,
                    move_played: *move_,
                    best_move: evaluation.best_move,
                    point_swing: evaluation.swing,
                });
            }
            
            if evaluation.swing < -15.0 {
                blunders.push(i as u32);
            }
        }
        
        GameReview {
            critical_moves,
            blunders,
            turning_point: self.find_turning_point(game),
            style_profile: self.analyze_style(game),
        }
    }
}

pub struct StyleAnalyzer {
    /// Classify player style from game
    pub fn analyze_player(&self, games: &[GameState]) -> PlayerProfile {
        let mut territorial_score = 0.0;
        let mut fighting_score = 0.0;
        let mut speed_score = 0.0;
        
        for game in games {
            let features = GameAnalyzer::extract_features(game);
            
            territorial_score += (100 - features.fighting_intensity) as f32;
            fighting_score += features.fighting_intensity as f32;
            speed_score += (200.0 / features.game_phase_transitions[2] as f32);
        }
        
        PlayerProfile {
            territorial: (territorial_score / games.len() as f32) as u8,
            fighting: (fighting_score / games.len() as f32) as u8,
            speed: (speed_score / games.len() as f32).min(100.0) as u8,
            consistency: self.calculate_consistency(games),
        }
    }
}
```

## 5. Burn Mechanism Design

The burn mechanism serves two purposes:
1. **Storage cleanup**: Remove old training data to free space
2. **Value recapture**: Recover deposits from stale data

```rust
impl TrainingMarketplace {
    /// Query burnable stamps for a relay
    #[ink(message)]
    pub fn get_burnable_stamps(&self, relay: AccountId) -> Vec<(Hash, Timestamp, Balance)> {
        let current_time = self.env().block_timestamp();
        let mut burnable = Vec::new();
        
        // Iterate through relay's accessible stamps
        // (In practice, this would need pagination)
        for (key, has_access) in self.relay_access.iter() {
            if key.0 == relay && has_access {
                if let Some(stamp) = self.training_stamps.get(&key.1) {
                    let age = current_time - stamp.timestamp;
                    if age > 30 * 24 * 60 * 60 * 1000 { // 30 days
                        let value = self.calculate_burn_value(&stamp);
                        burnable.push((key.1, stamp.timestamp, value));
                    }
                }
            }
        }
        
        burnable
    }
    
    /// Calculate burn value based on stamp usage
    fn calculate_burn_value(&self, stamp: &TrainingStamp) -> Balance {
        let base_value = 1_000_000; // Base deposit
        
        // Check if stamp was used in any successful models
        let usage_multiplier = self.get_stamp_usage_count(&stamp.game_hash);
        
        if usage_multiplier > 0 {
            // Reduce burn value if stamp was useful
            base_value / (usage_multiplier as u128 + 1)
        } else {
            // Full value if never used
            base_value
        }
    }
}
```

## Key Improvements:

1. **Training Stamps**: Both players' relays get access to consensus game data
2. **Batch Submission**: Reduces gas costs dramatically
3. **Feature Extraction**: Rich game analysis beyond win/loss
4. **Subsidy Pool**: Rewards quality without complex tokenomics
5. **Burn Mechanism**: Clean up old data, recover deposits
6. **Player Tools**: Style analysis, mistake detection, game review

The system naturally encourages quality:
- Only consensus games create stamps
- Better models get subsidies
- Old unused data can be burned for deposit recovery
- Batching makes it profitable even with gas costs