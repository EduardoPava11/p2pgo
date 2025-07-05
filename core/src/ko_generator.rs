use crate::ko_detector::{ContextMove, KoSituation};
use crate::{Color, Coord, GameState, Move};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Ko Pattern Generator - Creates synthetic Ko situations for training
/// when SGF files don't contain natural Ko situations
pub struct KoPatternGenerator {
    board_size: u8,
}

// Compatibility aliases
pub type KoGenerator = KoPatternGenerator;
pub type KoTrainingGenerator = KoPatternGenerator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedKoPattern {
    /// Pattern name
    pub name: String,
    /// Description
    pub description: String,
    /// Board setup moves
    pub setup_moves: Vec<Move>,
    /// Ko triggering move
    pub ko_trigger: Move,
    /// Expected Ko point
    pub ko_point: Coord,
    /// Suggested follow-up moves
    pub follow_up_moves: Vec<Move>,
    /// Pattern difficulty (1-5)
    pub difficulty: u8,
}

impl KoPatternGenerator {
    pub fn new(board_size: u8) -> Self {
        Self { board_size }
    }

    /// Generate all standard Ko patterns
    pub fn generate_all_patterns(&self) -> Vec<GeneratedKoPattern> {
        vec![
            self.generate_basic_ko(),
            self.generate_ladder_ko(),
            self.generate_double_ko(),
            self.generate_corner_ko(),
            self.generate_edge_ko(),
            self.generate_complex_ko(),
        ]
    }

    /// Basic Ko pattern
    fn generate_basic_ko(&self) -> GeneratedKoPattern {
        let center = self.board_size / 2;

        let setup_moves = vec![
            // Black stones forming Ko shape
            Move::Place {
                x: center - 1,
                y: center,
                color: Color::Black,
            },
            Move::Place {
                x: center + 1,
                y: center,
                color: Color::Black,
            },
            Move::Place {
                x: center,
                y: center - 1,
                color: Color::Black,
            },
            Move::Place {
                x: center,
                y: center + 1,
                color: Color::Black,
            },
            // White stones
            Move::Place {
                x: center,
                y: center,
                color: Color::White,
            },
            Move::Place {
                x: center - 2,
                y: center,
                color: Color::White,
            },
            Move::Place {
                x: center + 2,
                y: center,
                color: Color::White,
            },
            Move::Place {
                x: center,
                y: center - 2,
                color: Color::White,
            },
        ];

        GeneratedKoPattern {
            name: "Basic Ko".to_string(),
            description: "Simple Ko situation in the center".to_string(),
            setup_moves,
            ko_trigger: Move::Place {
                x: center,
                y: center + 2,
                color: Color::White,
            },
            ko_point: Coord::new(center, center),
            follow_up_moves: vec![
                Move::Place {
                    x: center + 3,
                    y: center,
                    color: Color::Black,
                }, // Ko threat
                Move::Place {
                    x: center + 3,
                    y: center + 1,
                    color: Color::White,
                }, // Response
                Move::Place {
                    x: center,
                    y: center,
                    color: Color::Black,
                }, // Recapture Ko
            ],
            difficulty: 1,
        }
    }

    /// Ladder Ko pattern
    fn generate_ladder_ko(&self) -> GeneratedKoPattern {
        let x = 3;
        let y = 3;

        let setup_moves = vec![
            // Create ladder shape
            Move::Place {
                x,
                y,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y,
                color: Color::White,
            },
            Move::Place {
                x,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x: x + 1,
                y: y + 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 2,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x: x + 1,
                y: y + 2,
                color: Color::White,
            },
            Move::Place {
                x: x + 2,
                y: y + 2,
                color: Color::Black,
            },
        ];

        GeneratedKoPattern {
            name: "Ladder Ko".to_string(),
            description: "Ko that appears during a ladder sequence".to_string(),
            setup_moves,
            ko_trigger: Move::Place {
                x: x + 3,
                y: y + 2,
                color: Color::White,
            },
            ko_point: Coord::new(x + 2, y + 2),
            follow_up_moves: vec![
                Move::Place {
                    x: x + 4,
                    y: y + 2,
                    color: Color::Black,
                },
                Move::Place {
                    x: x + 3,
                    y: y + 3,
                    color: Color::White,
                },
            ],
            difficulty: 3,
        }
    }

    /// Double Ko pattern
    fn generate_double_ko(&self) -> GeneratedKoPattern {
        let x = self.board_size / 2 - 2;
        let y = self.board_size / 2;

        let setup_moves = vec![
            // First Ko setup
            Move::Place {
                x,
                y,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y: y - 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y: y + 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y,
                color: Color::White,
            },
            // Second Ko setup
            Move::Place {
                x: x + 3,
                y,
                color: Color::White,
            },
            Move::Place {
                x: x + 4,
                y: y - 1,
                color: Color::White,
            },
            Move::Place {
                x: x + 4,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x: x + 4,
                y,
                color: Color::Black,
            },
            // Connecting stones
            Move::Place {
                x: x + 2,
                y: y - 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 2,
                y: y + 1,
                color: Color::White,
            },
        ];

        GeneratedKoPattern {
            name: "Double Ko".to_string(),
            description: "Two Ko situations that interact with each other".to_string(),
            setup_moves,
            ko_trigger: Move::Place {
                x: x + 2,
                y,
                color: Color::Black,
            },
            ko_point: Coord::new(x + 1, y),
            follow_up_moves: vec![
                Move::Place {
                    x: x + 5,
                    y,
                    color: Color::White,
                },
                Move::Place {
                    x: x + 4,
                    y,
                    color: Color::Black,
                }, // Second Ko capture
            ],
            difficulty: 5,
        }
    }

    /// Corner Ko pattern
    fn generate_corner_ko(&self) -> GeneratedKoPattern {
        let setup_moves = vec![
            // Corner setup
            Move::Place {
                x: 0,
                y: 1,
                color: Color::Black,
            },
            Move::Place {
                x: 1,
                y: 0,
                color: Color::Black,
            },
            Move::Place {
                x: 1,
                y: 1,
                color: Color::White,
            },
            Move::Place {
                x: 0,
                y: 0,
                color: Color::White,
            },
            Move::Place {
                x: 2,
                y: 0,
                color: Color::Black,
            },
            Move::Place {
                x: 0,
                y: 2,
                color: Color::Black,
            },
        ];

        GeneratedKoPattern {
            name: "Corner Ko".to_string(),
            description: "Ko situation in the corner of the board".to_string(),
            setup_moves,
            ko_trigger: Move::Place {
                x: 1,
                y: 2,
                color: Color::White,
            },
            ko_point: Coord::new(0, 0),
            follow_up_moves: vec![
                Move::Place {
                    x: 3,
                    y: 0,
                    color: Color::Black,
                }, // Ko threat
                Move::Place {
                    x: 3,
                    y: 1,
                    color: Color::White,
                },
                Move::Place {
                    x: 0,
                    y: 0,
                    color: Color::Black,
                }, // Recapture
            ],
            difficulty: 2,
        }
    }

    /// Edge Ko pattern
    fn generate_edge_ko(&self) -> GeneratedKoPattern {
        let y = self.board_size / 2;

        let setup_moves = vec![
            // Edge setup
            Move::Place {
                x: 0,
                y: y - 1,
                color: Color::Black,
            },
            Move::Place {
                x: 0,
                y: y + 1,
                color: Color::Black,
            },
            Move::Place {
                x: 1,
                y,
                color: Color::Black,
            },
            Move::Place {
                x: 0,
                y,
                color: Color::White,
            },
            Move::Place {
                x: 1,
                y: y - 1,
                color: Color::White,
            },
            Move::Place {
                x: 1,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x: 2,
                y,
                color: Color::White,
            },
        ];

        GeneratedKoPattern {
            name: "Edge Ko".to_string(),
            description: "Ko situation on the edge of the board".to_string(),
            setup_moves,
            ko_trigger: Move::Place {
                x: 2,
                y: y - 1,
                color: Color::Black,
            },
            ko_point: Coord::new(0, y),
            follow_up_moves: vec![
                Move::Place {
                    x: 3,
                    y,
                    color: Color::White,
                },
                Move::Place {
                    x: 0,
                    y,
                    color: Color::Black,
                },
            ],
            difficulty: 2,
        }
    }

    /// Complex fighting Ko
    fn generate_complex_ko(&self) -> GeneratedKoPattern {
        let x = self.board_size / 2 - 1;
        let y = self.board_size / 2 - 1;

        let setup_moves = vec![
            // Complex fighting shape
            Move::Place {
                x,
                y,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y,
                color: Color::White,
            },
            Move::Place {
                x: x + 2,
                y,
                color: Color::Black,
            },
            Move::Place {
                x,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x: x + 1,
                y: y + 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 2,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x,
                y: y + 2,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y: y + 2,
                color: Color::White,
            },
            Move::Place {
                x: x + 2,
                y: y + 2,
                color: Color::Black,
            },
            // Additional stones for complexity
            Move::Place {
                x: x - 1,
                y: y + 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 3,
                y: y + 1,
                color: Color::White,
            },
            Move::Place {
                x: x + 1,
                y: y - 1,
                color: Color::Black,
            },
            Move::Place {
                x: x + 1,
                y: y + 3,
                color: Color::White,
            },
        ];

        GeneratedKoPattern {
            name: "Complex Fighting Ko".to_string(),
            description: "Ko that arises from complex fighting".to_string(),
            setup_moves,
            ko_trigger: Move::Place {
                x: x + 1,
                y: y + 4,
                color: Color::Black,
            },
            ko_point: Coord::new(x + 1, y + 1),
            follow_up_moves: vec![
                Move::Place {
                    x: x + 4,
                    y: y + 1,
                    color: Color::White,
                },
                Move::Place {
                    x: x + 1,
                    y: y + 1,
                    color: Color::Black,
                },
            ],
            difficulty: 4,
        }
    }

    /// Convert pattern to Ko situation for training
    pub fn pattern_to_ko_situation(
        &self,
        pattern: &GeneratedKoPattern,
        start_move: usize,
    ) -> KoSituation {
        let context_moves: Vec<ContextMove> = pattern
            .setup_moves
            .iter()
            .chain(std::iter::once(&pattern.ko_trigger))
            .chain(pattern.follow_up_moves.iter())
            .map(|m| match m {
                Move::Place { x, y, color } => ContextMove {
                    color: *color,
                    coord: Some(Coord::new(*x, *y)),
                },
                _ => ContextMove {
                    color: Color::Black,
                    coord: None,
                },
            })
            .collect();

        KoSituation {
            start_move,
            capture_move: start_move + pattern.setup_moves.len(),
            recapture_move: Some(start_move + pattern.setup_moves.len() + 2),
            ko_point: pattern.ko_point,
            initiator: match pattern.ko_trigger {
                Move::Place { color, .. } => color,
                _ => Color::Black,
            },
            board_before: String::new(), // Will be filled when applied
            context_moves,
        }
    }
}

/// Apply Ko pattern to a game state
pub fn apply_ko_pattern(game_state: &mut GameState, pattern: &GeneratedKoPattern) -> Result<()> {
    // Apply setup moves
    for mv in &pattern.setup_moves {
        game_state.apply_move(mv.clone())?;
    }

    // Apply Ko trigger
    game_state.apply_move(pattern.ko_trigger.clone())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ko_pattern_generation() {
        let generator = KoPatternGenerator::new(19);
        let patterns = generator.generate_all_patterns();

        assert_eq!(patterns.len(), 6);

        for pattern in patterns {
            assert!(!pattern.name.is_empty());
            assert!(!pattern.setup_moves.is_empty());
            assert!(pattern.difficulty >= 1 && pattern.difficulty <= 5);
        }
    }
}
