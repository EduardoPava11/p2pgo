//! Neural network configuration based on user preferences

use serde::{Serialize, Deserialize};

/// Neural network configuration from questionnaire responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralConfig {
    /// Aggression level (1-10) - Higher means more aggressive play
    pub aggression: u8,
    
    /// Territory focus (1-10) - Higher means more territorial
    pub territory_focus: u8,
    
    /// Fighting spirit (1-10) - Higher means more willing to fight
    pub fighting_spirit: u8,
    
    /// Pattern recognition (1-10) - Higher means more pattern-based
    pub pattern_recognition: u8,
    
    /// Risk tolerance (1-10) - Higher means more risk-taking
    pub risk_tolerance: u8,
    
    /// Opening style (1-10) - 1=Defensive, 10=Aggressive
    pub opening_style: u8,
    
    /// Middle game focus (1-10) - Higher means stronger middle game
    pub middle_game_focus: u8,
    
    /// Endgame precision (1-10) - Higher means better endgame
    pub endgame_precision: u8,
    
    /// Learning rate (1-10) - Higher means faster adaptation
    pub learning_rate: u8,
    
    /// Creativity (1-10) - Higher means more unconventional moves
    pub creativity: u8,
}

impl Default for NeuralConfig {
    fn default() -> Self {
        Self {
            aggression: 5,
            territory_focus: 5,
            fighting_spirit: 5,
            pattern_recognition: 5,
            risk_tolerance: 5,
            opening_style: 5,
            middle_game_focus: 5,
            endgame_precision: 5,
            learning_rate: 5,
            creativity: 5,
        }
    }
}

impl NeuralConfig {
    /// Create a balanced configuration
    pub fn balanced() -> Self {
        Self::default()
    }
    
    /// Create an aggressive configuration
    pub fn aggressive() -> Self {
        Self {
            aggression: 8,
            territory_focus: 3,
            fighting_spirit: 9,
            pattern_recognition: 6,
            risk_tolerance: 8,
            opening_style: 8,
            middle_game_focus: 7,
            endgame_precision: 5,
            learning_rate: 7,
            creativity: 7,
        }
    }
    
    /// Create a territorial configuration
    pub fn territorial() -> Self {
        Self {
            aggression: 3,
            territory_focus: 9,
            fighting_spirit: 4,
            pattern_recognition: 8,
            risk_tolerance: 3,
            opening_style: 3,
            middle_game_focus: 6,
            endgame_precision: 8,
            learning_rate: 6,
            creativity: 4,
        }
    }
    
    /// Convert to neural network weights
    pub fn to_weights(&self) -> NeuralWeights {
        NeuralWeights {
            // Attack patterns get higher weight with more aggression
            attack_weight: self.aggression as f32 / 10.0,
            
            // Defense patterns get higher weight with less aggression
            defense_weight: (10 - self.aggression) as f32 / 10.0,
            
            // Territory evaluation
            territory_weight: self.territory_focus as f32 / 10.0,
            
            // Influence evaluation (opposite of territory)
            influence_weight: (10 - self.territory_focus) as f32 / 10.0,
            
            // Local fighting
            local_weight: self.fighting_spirit as f32 / 10.0,
            
            // Global planning
            global_weight: (10 - self.fighting_spirit) as f32 / 10.0,
            
            // Pattern matching strength
            pattern_weight: self.pattern_recognition as f32 / 10.0,
            
            // Calculation depth (risk tolerance affects reading)
            reading_depth: 1 + (self.risk_tolerance as usize / 3),
            
            // Opening preferences
            corner_preference: match self.opening_style {
                1..=3 => 0.9,  // Defensive - strong corner preference
                4..=7 => 0.7,  // Balanced
                _ => 0.5,      // Aggressive - less corner focus
            },
            
            // Game phase weights
            opening_weight: if self.opening_style >= 7 { 1.2 } else { 1.0 },
            middle_weight: self.middle_game_focus as f32 / 10.0 + 0.5,
            endgame_weight: self.endgame_precision as f32 / 10.0 + 0.5,
            
            // Learning parameters
            learning_rate: self.learning_rate as f32 / 100.0,
            exploration_rate: self.creativity as f32 / 20.0, // 0.05 to 0.5
        }
    }
}

/// Neural network weights derived from configuration
#[derive(Debug, Clone)]
pub struct NeuralWeights {
    pub attack_weight: f32,
    pub defense_weight: f32,
    pub territory_weight: f32,
    pub influence_weight: f32,
    pub local_weight: f32,
    pub global_weight: f32,
    pub pattern_weight: f32,
    pub reading_depth: usize,
    pub corner_preference: f32,
    pub opening_weight: f32,
    pub middle_weight: f32,
    pub endgame_weight: f32,
    pub learning_rate: f32,
    pub exploration_rate: f32,
}

/// Configuration wizard for interactive setup
pub struct ConfigWizard {
    questions: Vec<Question>,
    answers: Vec<u8>,
}

struct Question {
    text: &'static str,
    description: &'static str,
}

impl ConfigWizard {
    pub fn new() -> Self {
        Self {
            questions: vec![
                Question {
                    text: "How aggressive should the AI play?",
                    description: "1 = Very defensive, 10 = Very aggressive",
                },
                Question {
                    text: "How much should it focus on territory?",
                    description: "1 = Fight everywhere, 10 = Secure territory",
                },
                Question {
                    text: "Fighting spirit level?",
                    description: "1 = Avoid fights, 10 = Seek combat",
                },
                Question {
                    text: "Pattern recognition importance?",
                    description: "1 = Calculate everything, 10 = Trust patterns",
                },
                Question {
                    text: "Risk tolerance?",
                    description: "1 = Very safe, 10 = High risk",
                },
                Question {
                    text: "Opening style preference?",
                    description: "1 = Defensive/slow, 10 = Aggressive/fast",
                },
                Question {
                    text: "Middle game strength?",
                    description: "1 = Weak middle game, 10 = Strong middle game",
                },
                Question {
                    text: "Endgame precision?",
                    description: "1 = Rough endgame, 10 = Precise endgame",
                },
                Question {
                    text: "How fast should it learn?",
                    description: "1 = Slow/stable, 10 = Fast/adaptive",
                },
                Question {
                    text: "Creativity level?",
                    description: "1 = Standard moves, 10 = Creative/unusual",
                },
            ],
            answers: Vec::new(),
        }
    }
    
    pub fn get_question(&self, index: usize) -> Option<(&str, &str)> {
        self.questions.get(index)
            .map(|q| (q.text, q.description))
    }
    
    pub fn answer(&mut self, value: u8) {
        if value >= 1 && value <= 10 {
            self.answers.push(value);
        }
    }
    
    pub fn is_complete(&self) -> bool {
        self.answers.len() == self.questions.len()
    }
    
    pub fn build_config(&self) -> Option<NeuralConfig> {
        if !self.is_complete() {
            return None;
        }
        
        Some(NeuralConfig {
            aggression: self.answers[0],
            territory_focus: self.answers[1],
            fighting_spirit: self.answers[2],
            pattern_recognition: self.answers[3],
            risk_tolerance: self.answers[4],
            opening_style: self.answers[5],
            middle_game_focus: self.answers[6],
            endgame_precision: self.answers[7],
            learning_rate: self.answers[8],
            creativity: self.answers[9],
        })
    }
}