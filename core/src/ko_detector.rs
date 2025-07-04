use crate::{board::Board, Color, Move, Coord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Ko detection and analysis
#[derive(Debug, Clone)]
pub struct KoDetector {
    /// Board positions we've seen (serialized board -> move number)
    position_history: HashMap<String, usize>,
    /// Detected Ko situations
    ko_situations: Vec<KoSituation>,
    /// Current move number
    current_move: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoSituation {
    /// Move number where Ko started
    pub start_move: usize,
    /// Move number where Ko was captured
    pub capture_move: usize,
    /// Move number where Ko was recaptured (if it happened)
    pub recapture_move: Option<usize>,
    /// The point being fought over
    pub ko_point: Coord,
    /// Who initiated the Ko
    pub initiator: Color,
    /// Board state before Ko
    pub board_before: String,
    /// Context moves (5 before, 5 after)
    pub context_moves: Vec<ContextMove>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMove {
    pub color: Color,
    pub coord: Option<Coord>,
}

impl KoDetector {
    pub fn new() -> Self {
        Self {
            position_history: HashMap::new(),
            ko_situations: Vec::new(),
            current_move: 0,
        }
    }
    
    /// Process a move and detect Ko situations
    pub fn process_move(&mut self, board: &Board, m: &Move, captured_stones: &[Coord]) -> Option<KoSituation> {
        self.current_move += 1;
        
        // Extract coord from Move
        let move_coord = match m {
            Move::Place { x, y, color: _ } => Some(Coord::new(*x, *y)),
            _ => None,
        };
        
        // Check if this is a Ko capture (single stone capture)
        if captured_stones.len() == 1 {
            let captured_coord = captured_stones[0];
            
            if let (Some(coord), Move::Place { color, .. }) = (move_coord, m) {
                // Check if the captured stone could immediately recapture
                if self.is_potential_ko(board, coord, captured_coord, *color) {
                    // Record potential Ko
                    let ko_situation = KoSituation {
                        start_move: self.current_move - 1,
                        capture_move: self.current_move,
                        recapture_move: None,
                        ko_point: captured_coord,
                        initiator: *color,
                        board_before: self.serialize_board(board),
                        context_moves: Vec::new(),
                    };
                    
                    self.ko_situations.push(ko_situation.clone());
                    return Some(ko_situation);
                }
            }
        }
        
        // Check if this move recaptures a Ko
        if let (Some(coord), Move::Place { color, .. }) = (move_coord, m) {
            for ko in &mut self.ko_situations {
                if ko.recapture_move.is_none() && 
                   coord == ko.ko_point && 
                   *color != ko.initiator {
                    ko.recapture_move = Some(self.current_move);
                }
            }
        }
        
        // Store board position
        let board_str = self.serialize_board(board);
        self.position_history.insert(board_str, self.current_move);
        
        None
    }
    
    /// Check if a position could lead to Ko
    fn is_potential_ko(&self, board: &Board, capture_coord: Coord, captured_coord: Coord, color: Color) -> bool {
        // A Ko situation exists when:
        // 1. We just captured a single stone
        // 2. That stone could immediately recapture our stone
        // 3. The resulting position would repeat
        
        // Check if the captured point is surrounded by our color except for the capture point
        let neighbors = captured_coord.adjacent_coords();
        
        // Count how many neighbors are our color
        let our_neighbors = neighbors.iter()
            .filter(|&&coord| coord.is_valid(board.size()) && board.get(coord) == Some(color) && coord != capture_coord)
            .count();
        
        // In a Ko, the captured stone should be surrounded by our stones
        our_neighbors >= neighbors.len() - 1
    }
    
    /// Get all detected Ko situations
    pub fn get_ko_situations(&self) -> &[KoSituation] {
        &self.ko_situations
    }
    
    /// Get Ko situations with context moves
    pub fn get_ko_with_context(&self, moves: &[Move], context_size: usize) -> Vec<KoSituation> {
        self.ko_situations.iter().map(|ko| {
            let start_idx = ko.start_move.saturating_sub(context_size);
            let end_idx = (ko.recapture_move.unwrap_or(ko.capture_move) + context_size).min(moves.len());
            
            let mut ko_with_context = ko.clone();
            ko_with_context.context_moves = moves[start_idx..end_idx].iter().map(|m| {
                match m {
                    Move::Place { x, y, color } => ContextMove {
                        color: *color,
                        coord: Some(Coord::new(*x, *y)),
                    },
                    _ => ContextMove {
                        color: Color::Black, // Default
                        coord: None,
                    }
                }
            }).collect();
            ko_with_context
        }).collect()
    }
    
    /// Serialize board state for comparison
    fn serialize_board(&self, board: &Board) -> String {
        let mut result = String::new();
        for y in 0..board.size() {
            for x in 0..board.size() {
                let coord = Coord::new(x, y);
                match board.get(coord) {
                    Some(Color::Black) => result.push('B'),
                    Some(Color::White) => result.push('W'),
                    None => result.push('.'),
                }
            }
        }
        result
    }
}

/// Ko sequence analyzer for training data
pub struct KoSequenceAnalyzer {
    /// Minimum moves before Ko to include
    pub context_before: usize,
    /// Minimum moves after Ko to include  
    pub context_after: usize,
}

impl KoSequenceAnalyzer {
    pub fn new(context_before: usize, context_after: usize) -> Self {
        Self {
            context_before,
            context_after,
        }
    }
    
    /// Extract training sequences around Ko situations
    pub fn extract_ko_sequences(&self, ko_situations: &[KoSituation], all_moves: &[Move]) -> Vec<KoTrainingSequence> {
        ko_situations.iter().filter_map(|ko| {
            let start = ko.start_move.saturating_sub(self.context_before);
            let end = if let Some(recap) = ko.recapture_move {
                (recap + self.context_after).min(all_moves.len())
            } else {
                (ko.capture_move + self.context_after).min(all_moves.len())
            };
            
            if end > start {
                let context_moves = all_moves[start..end].iter().map(|m| {
                    match m {
                        Move::Place { x, y, color } => ContextMove {
                            color: *color,
                            coord: Some(Coord::new(*x, *y)),
                        },
                        _ => ContextMove {
                            color: Color::Black,
                            coord: None,
                        }
                    }
                }).collect();
                
                Some(KoTrainingSequence {
                    ko_situation: ko.clone(),
                    sequence_moves: context_moves,
                    start_move_num: start,
                    end_move_num: end,
                })
            } else {
                None
            }
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoTrainingSequence {
    pub ko_situation: KoSituation,
    pub sequence_moves: Vec<ContextMove>,
    pub start_move_num: usize,
    pub end_move_num: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ko_detection() {
        let mut board = Board::new(9);
        let mut detector = KoDetector::new();
        
        // Set up a Ko situation
        // Black stones
        board.place(Coord::new(3, 3), Color::Black);
        board.place(Coord::new(4, 2), Color::Black);
        board.place(Coord::new(5, 3), Color::Black);
        board.place(Coord::new(4, 4), Color::Black);
        
        // White stones
        board.place(Coord::new(4, 3), Color::White);
        board.place(Coord::new(3, 2), Color::White);
        board.place(Coord::new(2, 3), Color::White);
        board.place(Coord::new(3, 4), Color::White);
        
        // Black captures at (3,3) - creating Ko
        let m = Move::Place { x: 3, y: 3, color: Color::Black };
        let captured = vec![Coord::new(4, 3)]; // Simulate capturing the white stone
        let ko = detector.process_move(&board, &m, &captured);
        
        assert!(ko.is_some());
        assert_eq!(detector.get_ko_situations().len(), 1);
    }
}