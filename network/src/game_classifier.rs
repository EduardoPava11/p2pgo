//! Game classification for federated learning value assessment

use serde::{Serialize, Deserialize};
use p2pgo_core::{GameState, Move};
use std::collections::HashMap;
use anyhow::Result;

/// Classification of a game for federated learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedGame {
    /// Game identifier
    pub game_id: String,
    /// Game type classification
    pub game_type: GameType,
    /// Teaching value score (0.0 - 1.0)
    pub teaching_value: f32,
    /// Specific lessons identified
    pub lessons: Vec<Lesson>,
    /// Critical turning points
    pub turning_points: Vec<TurningPoint>,
    /// Player information
    pub players: PlayerInfo,
    /// Game metrics
    pub metrics: GameMetrics,
}

/// Type of game for training purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    /// High ELO differential, low variance - shows what TO do
    Teaching,
    /// Low ELO differential, high variance - shows fighting spirit
    Dogfight,
    /// Standard game with moderate teaching value
    Standard,
    /// Low quality game with little teaching value
    LowValue,
}

/// Specific lessons demonstrated in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Lesson {
    /// Opening theory demonstration
    OpeningTheory { 
        joseki_name: String,
        variation: String,
        accuracy: f32,
    },
    /// Middle game strategic concept
    MiddleGameStrategy { 
        concept: String,
        demonstration_quality: f32,
    },
    /// Endgame technique
    EndgameTechnique {
        technique_type: String,
        points_gained: i32,
    },
    /// Tactical sequence
    TacticalSequence {
        sequence_length: u32,
        complexity_score: f32,
        success: bool,
    },
    /// Life and death problem
    LifeAndDeath {
        group_size: u32,
        difficulty: f32,
        correct_solution: bool,
    },
}

/// Critical turning point in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPoint {
    /// Move number where turning point occurred
    pub move_number: u32,
    /// Evaluation change (positive favors black)
    pub evaluation_swing: f32,
    /// Type of mistake or brilliant move
    pub move_classification: MoveClassification,
    /// How the player responded
    pub recovery_quality: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoveClassification {
    Brilliant,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
    Recovery,
}

/// Player information for the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// Black player ELO
    pub black_elo: Option<i32>,
    /// White player ELO
    pub white_elo: Option<i32>,
    /// ELO differential
    pub elo_differential: i32,
    /// Historical performance between these players
    pub head_to_head: HeadToHeadStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadToHeadStats {
    /// Previous games played
    pub games_played: u32,
    /// Win rate for black
    pub black_wins: u32,
    /// Average game length
    pub avg_game_length: f32,
}

/// Metrics for game quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetrics {
    /// Total moves in the game
    pub total_moves: u32,
    /// Number of mistakes by each player
    pub mistakes: (u32, u32), // (black, white)
    /// Variance in evaluation throughout the game
    pub evaluation_variance: f32,
    /// Percentage of moves matching AI recommendations
    pub ai_agreement_rate: f32,
    /// Game phase distribution
    pub phase_distribution: PhaseDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseDistribution {
    /// Percentage of moves in opening
    pub opening: f32,
    /// Percentage in middle game
    pub middle_game: f32,
    /// Percentage in endgame
    pub endgame: f32,
}

/// Game classifier for federated learning
pub struct GameClassifier {
    /// Neural network for position evaluation
    evaluator: Box<dyn PositionEvaluator>,
    /// Opening book database
    opening_book: OpeningBook,
    /// Pattern matcher for lessons
    _pattern_matcher: PatternMatcher,
}

/// Trait for position evaluation
pub trait PositionEvaluator: Send + Sync {
    /// Evaluate a position
    fn evaluate(&self, state: &GameState) -> f32;
    
    /// Get best moves for a position
    fn get_best_moves(&self, state: &GameState, count: usize) -> Vec<(Move, f32)>;
}

/// Opening book for joseki detection
pub struct OpeningBook {
    /// Known joseki patterns
    _joseki_database: HashMap<String, JosekiPattern>,
}

#[derive(Debug, Clone)]
pub struct JosekiPattern {
    pub name: String,
    pub moves: Vec<Move>,
    pub variations: Vec<Vec<Move>>,
}

/// Pattern matcher for identifying lessons
pub struct PatternMatcher {
    /// Tactical patterns
    _tactical_patterns: Vec<TacticalPattern>,
    /// Strategic patterns
    _strategic_patterns: Vec<StrategicPattern>,
}

#[derive(Debug, Clone)]
pub struct TacticalPattern {
    pub name: String,
    pub pattern_type: String,
    pub complexity: f32,
}

#[derive(Debug, Clone)]
pub struct StrategicPattern {
    pub concept: String,
    pub evaluation_criteria: Vec<String>,
}

impl GameClassifier {
    /// Create a new game classifier
    pub fn new(evaluator: Box<dyn PositionEvaluator>) -> Self {
        Self {
            evaluator,
            opening_book: OpeningBook::default(),
            _pattern_matcher: PatternMatcher::default(),
        }
    }
    
    /// Classify a completed game
    pub fn classify_game(&self, game_record: &GameRecord) -> Result<ClassifiedGame> {
        // Extract player information
        let players = self.extract_player_info(game_record)?;
        
        // Analyze game progression
        let (metrics, turning_points) = self.analyze_game_progression(game_record)?;
        
        // Identify lessons
        let lessons = self.identify_lessons(game_record)?;
        
        // Determine game type
        let game_type = self.determine_game_type(&players, &metrics);
        
        // Calculate teaching value
        let teaching_value = self.calculate_teaching_value(
            &game_type,
            &lessons,
            &turning_points,
            &metrics,
        );
        
        Ok(ClassifiedGame {
            game_id: game_record.game_id.clone(),
            game_type,
            teaching_value,
            lessons,
            turning_points,
            players,
            metrics,
        })
    }
    
    /// Extract player information from game record
    fn extract_player_info(&self, game_record: &GameRecord) -> Result<PlayerInfo> {
        let elo_differential = (game_record.black_elo.unwrap_or(1500) - 
                               game_record.white_elo.unwrap_or(1500)).abs();
        
        Ok(PlayerInfo {
            black_elo: game_record.black_elo,
            white_elo: game_record.white_elo,
            elo_differential,
            head_to_head: HeadToHeadStats {
                games_played: 0, // Would query historical data
                black_wins: 0,
                avg_game_length: 0.0,
            },
        })
    }
    
    /// Analyze game progression for metrics and turning points
    fn analyze_game_progression(
        &self,
        game_record: &GameRecord,
    ) -> Result<(GameMetrics, Vec<TurningPoint>)> {
        let mut evaluations = Vec::new();
        let mut mistakes = (0u32, 0u32);
        let mut turning_points = Vec::new();
        let mut ai_matches = 0u32;
        
        // Replay game and evaluate each position
        let mut state = GameState::new(game_record.board_size);
        
        for (i, game_move) in game_record.moves.iter().enumerate() {
            // Get AI recommendation
            let best_moves = self.evaluator.get_best_moves(&state, 3);
            
            // Check if move matches AI
            if best_moves.iter().any(|(m, _)| m == game_move) {
                ai_matches += 1;
            }
            
            // Apply move
            let _ = state.apply_move(game_move.clone())?;
            
            // Evaluate position
            let eval = self.evaluator.evaluate(&state);
            
            // Check for turning points
            if i > 0 {
                let eval_change = eval - evaluations.last().unwrap();
                if eval_change.abs() > 10.0 {
                    turning_points.push(TurningPoint {
                        move_number: i as u32,
                        evaluation_swing: eval_change,
                        move_classification: self.classify_move(eval_change),
                        recovery_quality: None, // Calculate in next moves
                    });
                }
                
                // Count mistakes
                if eval_change.abs() > 5.0 {
                    if i % 2 == 0 && eval_change < 0.0 {
                        mistakes.0 += 1; // Black mistake
                    } else if i % 2 == 1 && eval_change > 0.0 {
                        mistakes.1 += 1; // White mistake
                    }
                }
            }
            
            evaluations.push(eval);
        }
        
        // Calculate variance
        let mean_eval = evaluations.iter().sum::<f32>() / evaluations.len() as f32;
        let variance = evaluations.iter()
            .map(|e| (e - mean_eval).powi(2))
            .sum::<f32>() / evaluations.len() as f32;
        
        let metrics = GameMetrics {
            total_moves: game_record.moves.len() as u32,
            mistakes,
            evaluation_variance: variance.sqrt(),
            ai_agreement_rate: ai_matches as f32 / game_record.moves.len() as f32,
            phase_distribution: self.calculate_phase_distribution(game_record.moves.len()),
        };
        
        Ok((metrics, turning_points))
    }
    
    /// Identify specific lessons in the game
    fn identify_lessons(&self, game_record: &GameRecord) -> Result<Vec<Lesson>> {
        let mut lessons = Vec::new();
        
        // Check opening
        if let Some(joseki) = self.opening_book.identify_joseki(&game_record.moves[..20.min(game_record.moves.len())]) {
            lessons.push(Lesson::OpeningTheory {
                joseki_name: joseki.name,
                variation: "Main".to_string(),
                accuracy: 0.9, // Calculate based on deviation
            });
        }
        
        // Check for tactical sequences
        // This would use pattern matching on the game
        
        // Check endgame
        if game_record.moves.len() > 150 {
            // Analyze endgame quality
            lessons.push(Lesson::EndgameTechnique {
                technique_type: "Counting".to_string(),
                points_gained: 5,
            });
        }
        
        Ok(lessons)
    }
    
    /// Determine game type based on metrics
    fn determine_game_type(&self, players: &PlayerInfo, metrics: &GameMetrics) -> GameType {
        if players.elo_differential > 200 && metrics.evaluation_variance < 10.0 {
            GameType::Teaching
        } else if players.elo_differential < 50 && metrics.evaluation_variance > 20.0 {
            GameType::Dogfight
        } else if metrics.ai_agreement_rate > 0.7 {
            GameType::Standard
        } else {
            GameType::LowValue
        }
    }
    
    /// Calculate teaching value score
    fn calculate_teaching_value(
        &self,
        game_type: &GameType,
        lessons: &[Lesson],
        turning_points: &[TurningPoint],
        metrics: &GameMetrics,
    ) -> f32 {
        let base_value = match game_type {
            GameType::Teaching => 0.8,
            GameType::Dogfight => 0.6,
            GameType::Standard => 0.4,
            GameType::LowValue => 0.1,
        };
        
        // Bonus for clear lessons
        let lesson_bonus = (lessons.len() as f32 * 0.05).min(0.2);
        
        // Bonus for interesting turning points
        let turning_bonus = (turning_points.len() as f32 * 0.02).min(0.1);
        
        // Penalty for too many mistakes
        let mistake_penalty = ((metrics.mistakes.0 + metrics.mistakes.1) as f32 * 0.01).min(0.2);
        
        (base_value + lesson_bonus + turning_bonus - mistake_penalty).clamp(0.0, 1.0)
    }
    
    /// Classify a move based on evaluation change
    fn classify_move(&self, eval_change: f32) -> MoveClassification {
        match eval_change.abs() {
            x if x < 2.0 => MoveClassification::Good,
            x if x < 5.0 => MoveClassification::Inaccuracy,
            x if x < 10.0 => MoveClassification::Mistake,
            x if x < 20.0 => MoveClassification::Blunder,
            _ => {
                if eval_change > 0.0 {
                    MoveClassification::Brilliant
                } else {
                    MoveClassification::Blunder
                }
            }
        }
    }
    
    /// Calculate game phase distribution
    fn calculate_phase_distribution(&self, total_moves: usize) -> PhaseDistribution {
        let opening_moves = 30.min(total_moves);
        let endgame_moves = if total_moves > 150 { total_moves - 150 } else { 0 };
        let middle_moves = total_moves - opening_moves - endgame_moves;
        
        PhaseDistribution {
            opening: opening_moves as f32 / total_moves as f32,
            middle_game: middle_moves as f32 / total_moves as f32,
            endgame: endgame_moves as f32 / total_moves as f32,
        }
    }
}

/// Game record for classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub game_id: String,
    pub board_size: u8,
    pub moves: Vec<Move>,
    pub black_elo: Option<i32>,
    pub white_elo: Option<i32>,
    pub result: GameResult,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameResult {
    BlackWin,
    WhiteWin,
    Draw,
}

impl OpeningBook {
    /// Identify joseki pattern from move sequence
    pub fn identify_joseki(&self, moves: &[Move]) -> Option<JosekiPattern> {
        // Simple pattern matching - in reality this would be more sophisticated
        if moves.len() >= 3 {
            // Return a basic pattern for any sequence
            Some(JosekiPattern {
                name: "Basic Pattern".to_string(),
                moves: moves.to_vec(),
                variations: vec![],
            })
        } else {
            None
        }
    }
}

impl Default for OpeningBook {
    fn default() -> Self {
        // Load common joseki patterns
        let mut joseki_database = HashMap::new();
        
        // Add some basic patterns
        joseki_database.insert(
            "3-3-invasion".to_string(),
            JosekiPattern {
                name: "3-3 Invasion".to_string(),
                moves: vec![], // Would contain actual moves
                variations: vec![],
            },
        );
        
        Self { _joseki_database: joseki_database }
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self {
            _tactical_patterns: vec![],
            _strategic_patterns: vec![],
        }
    }
}