// SPDX-License-Identifier: MIT OR Apache-2.0

//! Guild System for P2P Go
//! 
//! Players are classified into guilds based on their play style:
//! - Activity: Forward-moving, aggressive play
//! - Reactivity: Response-based, defensive play  
//! - Avoidance: Balance-seeking, territorial play

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The three orthogonal play styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Guild {
    /// Activity - measures forward vectors from previous stone
    Activity,
    /// Reactivity - measures backward vectors to previous stone
    Reactivity,
    /// Avoidance - seeks midpoint balance
    Avoidance,
}

/// Vector measurement between stones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoneVector {
    /// From position (previous stone or capture point)
    pub from: (u8, u8),
    /// To position (newly placed stone)
    pub to: (u8, u8),
    /// Whether 'from' was a capture point
    pub from_capture: bool,
}

impl StoneVector {
    /// Calculate the vector magnitude
    pub fn magnitude(&self) -> f32 {
        let dx = self.to.0 as f32 - self.from.0 as f32;
        let dy = self.to.1 as f32 - self.from.1 as f32;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Calculate guild affinity based on vector interpretation
    pub fn guild_affinity(&self) -> HashMap<Guild, f32> {
        let dx = self.to.0 as f32 - self.from.0 as f32;
        let dy = self.to.1 as f32 - self.from.1 as f32;
        
        // Activity: forward vector (from -> to)
        let activity_score = (dx.abs() + dy.abs()) / 2.0;
        
        // Reactivity: backward vector (to -> from)  
        let reactivity_score = if self.from_capture {
            // Strong reaction to captures
            activity_score * 1.5
        } else {
            activity_score * 0.8
        };
        
        // Avoidance: distance from midpoint
        let mid_x = (self.from.0 + self.to.0) as f32 / 2.0;
        let mid_y = (self.from.1 + self.to.1) as f32 / 2.0;
        let board_center = 4.0; // For 9x9 board
        let distance_from_center = ((mid_x - board_center).abs() + (mid_y - board_center).abs()) / 2.0;
        let avoidance_score = 1.0 / (1.0 + distance_from_center);
        
        let mut scores = HashMap::new();
        scores.insert(Guild::Activity, activity_score);
        scores.insert(Guild::Reactivity, reactivity_score);
        scores.insert(Guild::Avoidance, avoidance_score);
        
        // Normalize scores
        let total: f32 = scores.values().sum();
        for score in scores.values_mut() {
            *score /= total;
        }
        
        scores
    }
}

/// Hidden layer rules for guild classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildClassifier {
    /// Layer weights for each guild
    pub layer_weights: HashMap<Guild, Vec<f32>>,
    /// Distance-based features
    pub distance_features: DistanceFeatures,
    /// Pattern recognition for each guild
    pub pattern_affinity: HashMap<Guild, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceFeatures {
    /// Distance from last placed stone
    pub from_last_stone: f32,
    /// Distance from last capture (if any)
    pub from_last_capture: Option<f32>,
    /// Distance from board center
    pub from_center: f32,
    /// Distance from nearest friendly stone
    pub from_nearest_friend: f32,
    /// Distance from nearest enemy stone
    pub from_nearest_enemy: f32,
}

impl GuildClassifier {
    /// Classify a move into guild preference
    pub fn classify_move(&self, vector: &StoneVector, features: &DistanceFeatures) -> Guild {
        let affinities = vector.guild_affinity();
        
        // Weight by distance features
        let mut weighted_scores = HashMap::new();
        
        for (guild, base_score) in affinities {
            let weight = match guild {
                Guild::Activity => {
                    // Activity players prefer closer to enemy
                    1.0 / (1.0 + features.from_nearest_enemy)
                }
                Guild::Reactivity => {
                    // Reactivity players respond to captures
                    if features.from_last_capture.is_some() {
                        2.0
                    } else {
                        1.0 / (1.0 + features.from_last_stone)
                    }
                }
                Guild::Avoidance => {
                    // Avoidance players prefer balance
                    let center_weight = 1.0 / (1.0 + (features.from_center - 4.0).abs());
                    let friend_weight = 1.0 / (1.0 + features.from_nearest_friend);
                    (center_weight + friend_weight) / 2.0
                }
            };
            
            weighted_scores.insert(guild, base_score * weight);
        }
        
        // Return guild with highest score
        weighted_scores.into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(guild, _)| guild)
            .unwrap_or(Guild::Activity)
    }
}

/// Relay fuel system using credits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayFuel {
    /// Credits available for relay hops
    pub credits: u64,
    /// Guild membership affects fuel efficiency
    pub guild: Guild,
    /// Network friends (other relays)
    pub friends: Vec<String>,
}

impl RelayFuel {
    /// Create new fuel system for a player
    pub fn new(guild: Guild) -> Self {
        Self {
            credits: 10, // Start with 10 hops
            guild,
            friends: Vec::new(),
        }
    }
    
    /// Consume fuel for a relay hop
    pub fn consume_hop(&mut self) -> Result<(), &'static str> {
        if self.credits == 0 {
            return Err("No fuel credits remaining");
        }
        self.credits -= 1;
        Ok(())
    }
    
    /// Add a new friend (costs 1 credit)
    pub fn add_friend(&mut self, relay_id: String) -> Result<(), &'static str> {
        if self.credits == 0 {
            return Err("No fuel credits for new friend");
        }
        self.credits -= 1;
        self.friends.push(relay_id);
        Ok(())
    }
    
    /// Earn credits based on guild activity
    pub fn earn_credits(&mut self, amount: u64) {
        self.credits += amount;
    }
}

/// Best play activation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPlayTracker {
    /// Total uses available
    pub total_uses: u8,
    /// Uses remaining
    pub uses_remaining: u8,
    /// Move numbers when activated
    pub activation_moves: Vec<u16>,
    /// Specialized phase (opening/middle/endgame)
    pub specialization: GamePhase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GamePhase {
    Opening,   // Moves 1-20
    Middle,    // Moves 21-60
    Endgame,   // Moves 61+
}

impl BestPlayTracker {
    pub fn new(specialization: GamePhase, total_uses: u8) -> Self {
        Self {
            total_uses,
            uses_remaining: total_uses,
            activation_moves: Vec::new(),
            specialization,
        }
    }
    
    /// Try to use best play hint
    pub fn use_hint(&mut self, move_number: u16) -> Result<(), &'static str> {
        if self.uses_remaining == 0 {
            return Err("No best play hints remaining");
        }
        
        // Check if we're in the right phase
        let current_phase = match move_number {
            1..=20 => GamePhase::Opening,
            21..=60 => GamePhase::Middle,
            _ => GamePhase::Endgame,
        };
        
        // Specialized bots work best in their phase
        let efficiency = match (self.specialization, current_phase) {
            (a, b) if std::mem::discriminant(&a) == std::mem::discriminant(&b) => 1.0,
            _ => 0.5, // Half effectiveness outside specialization
        };
        
        if efficiency < 1.0 && self.uses_remaining == 1 {
            return Err("Save last hint for specialized phase");
        }
        
        self.uses_remaining -= 1;
        self.activation_moves.push(move_number);
        Ok(())
    }
}

/// Three-player higher order node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HigherOrderNode {
    /// The three players must each be from different guilds
    pub players: HashMap<Guild, String>,
    /// Combined ELO rating
    pub combined_elo: u32,
    /// Entropy reward multiplier
    pub entropy_multiplier: f32,
}

impl HigherOrderNode {
    /// Try to form a higher order node
    pub fn try_form(players: Vec<(String, Guild, u32)>) -> Result<Self, &'static str> {
        if players.len() != 3 {
            return Err("Exactly 3 players required");
        }
        
        let mut guild_map = HashMap::new();
        let mut total_elo = 0;
        
        for (player_id, guild, elo) in players {
            if guild_map.contains_key(&guild) {
                return Err("All players must be from different guilds");
            }
            guild_map.insert(guild, player_id);
            total_elo += elo;
        }
        
        if guild_map.len() != 3 {
            return Err("Missing guild representation");
        }
        
        Ok(Self {
            players: guild_map,
            combined_elo: total_elo / 3,
            entropy_multiplier: 1.5, // 50% bonus for diversity
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stone_vector_guild_affinity() {
        let vector = StoneVector {
            from: (2, 2),
            to: (5, 5),
            from_capture: false,
        };
        
        let affinities = vector.guild_affinity();
        assert_eq!(affinities.len(), 3);
        
        // All affinities should sum to 1.0
        let sum: f32 = affinities.values().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_relay_fuel() {
        let mut fuel = RelayFuel::new(Guild::Activity);
        assert_eq!(fuel.credits, 10);
        
        assert!(fuel.consume_hop().is_ok());
        assert_eq!(fuel.credits, 9);
        
        assert!(fuel.add_friend("relay_123".to_string()).is_ok());
        assert_eq!(fuel.credits, 8);
        assert_eq!(fuel.friends.len(), 1);
    }
    
    #[test]
    fn test_best_play_tracker() {
        let mut tracker = BestPlayTracker::new(GamePhase::Opening, 3);
        
        assert!(tracker.use_hint(5).is_ok());
        assert_eq!(tracker.uses_remaining, 2);
        assert_eq!(tracker.activation_moves, vec![5]);
        
        // Using hint in wrong phase
        assert!(tracker.use_hint(50).is_ok()); // Works but less effective
        assert_eq!(tracker.uses_remaining, 1);
    }
    
    #[test]
    fn test_higher_order_node() {
        let players = vec![
            ("alice".to_string(), Guild::Activity, 1500),
            ("bob".to_string(), Guild::Reactivity, 1600),
            ("carol".to_string(), Guild::Avoidance, 1400),
        ];
        
        let node = HigherOrderNode::try_form(players).unwrap();
        assert_eq!(node.combined_elo, 1500);
        assert_eq!(node.entropy_multiplier, 1.5);
    }
}