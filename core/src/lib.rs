// SPDX-License-Identifier: MIT OR Apache-2.0

#![deny(warnings)]
#![deny(unsafe_code)]
#![deny(clippy::all)]

//! P2P Go Core - Game Rules and Board Logic
//!
//! This crate provides the core game functionality including:
//! - Go board representation and manipulation
//! - Game rules and validation
//! - SGF (Smart Game Format) parsing and generation
//! - CBOR serialization helpers for game state

pub mod board;
pub mod rules;
pub mod sgf;
pub mod sgf_parser;
pub mod cbor;
pub mod engine;
pub mod value_labeller;
pub mod scoring;
pub mod archiver;
pub mod color_constants;
pub mod burn_engine;
pub mod training_pipeline;
pub mod ko_detector;
pub mod ko_generator;

use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::board::Board;

/// Player color in a Go game (Black or White)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    /// Black player (traditionally goes first)
    Black,
    /// White player
    White,
}

impl Color {
    /// Returns the opposite color
    pub fn opposite(&self) -> Self {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

/// Board coordinate representing a position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coord {
    /// X coordinate (column)
    pub x: u8,
    /// Y coordinate (row)
    pub y: u8,
}

impl Coord {
    /// Create a new coordinate
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
    
    /// Check if coordinate is valid for a board of given size
    pub fn is_valid(&self, board_size: u8) -> bool {
        self.x < board_size && self.y < board_size
    }
    
    /// Get adjacent (neighboring) coordinates in the four cardinal directions
    pub fn adjacent_coords(&self) -> Vec<Coord> {
        let mut neighbors = Vec::with_capacity(4);
        
        // Check north
        if self.y > 0 {
            neighbors.push(Coord::new(self.x, self.y - 1));
        }
        
        // Check east
        neighbors.push(Coord::new(self.x + 1, self.y));
        
        // Check south
        neighbors.push(Coord::new(self.x, self.y + 1));
        
        // Check west
        if self.x > 0 {
            neighbors.push(Coord::new(self.x - 1, self.y));
        }
        
        neighbors
    }
}

/// Represents a move in the game
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Move {
    /// Place a stone at the specified coordinate
    Place { x: u8, y: u8, color: Color },
    /// Pass the turn
    Pass,
    /// Resign the game
    Resign,
}

/// Game result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameResult {
    /// Black wins by score
    BlackWin(f32),
    /// White wins by score
    WhiteWin(f32),
    /// Black wins by resignation
    BlackWinByResignation,
    /// White wins by resignation
    WhiteWinByResignation,
    /// Draw (very rare in Go)
    Draw,
}

/// Represents the current state of a Go game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Unique game identifier
    pub id: String,
    /// The size of the board (typically 9, 13, or 19)
    pub board_size: u8,
    /// The current board positions
    pub board: Vec<Option<Color>>,
    /// The player whose turn it is
    pub current_player: Color,
    /// History of moves
    pub moves: Vec<Move>,
    /// Number of consecutive passes
    pub pass_count: u8,
    /// Captured stones count for each player
    pub captures: (u16, u16), // (Black captures, White captures)
    /// Game result (if finished)
    pub result: Option<GameResult>,
}

impl GameState {
    /// Create a new game with the specified board size
    pub fn new(board_size: u8) -> Self {
        let board_cells = (board_size as usize) * (board_size as usize);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            board_size,
            board: vec![None; board_cells],
            current_player: Color::Black, // Black goes first
            moves: Vec::new(),
            pass_count: 0,
            captures: (0, 0),
            result: None,
        }
    }
    
    /// Apply a move to the game state
    pub fn apply_move(&mut self, mv: Move) -> Result<Vec<GameEvent>, GameError> {
        let mut events = Vec::new();
        
        match mv {
            Move::Place { x, y, color } => {
                if x >= self.board_size || y >= self.board_size {
                    return Err(GameError::InvalidCoordinate);
                }
                
                let coord = Coord::new(x, y);
                let idx = (y as usize) * (self.board_size as usize) + (x as usize);
                if self.board[idx].is_some() {
                    return Err(GameError::OccupiedPosition);
                }
                
                // Create board for rules checking
                let mut board = self.to_board();
                let prev_board = board.clone();
                
                // Check if move is valid (suicide, ko)
                {
                    let validator = crate::rules::RuleValidator::new(&board, &prev_board);
                    validator.check_move(coord, color)?;
                }
                
                // Place the stone
                self.board[idx] = Some(color);
                board.place(coord, color);
                
                // Find and remove captured stones
                let validator = crate::rules::RuleValidator::new(&board, &prev_board);
                let captured_positions = validator.find_captures(coord);
                let mut total_captured = 0;
                
                if !captured_positions.is_empty() {
                    total_captured = captured_positions.len() as u16;
                    
                    // Remove captured stones
                    for pos in &captured_positions {
                        let cap_idx = (pos.y as usize) * (self.board_size as usize) + (pos.x as usize);
                        self.board[cap_idx] = None;
                    }
                    
                    // Emit capture event
                    let captured_color = color.opposite();
                    events.push(GameEvent::StonesCaptured {
                        count: captured_positions.len() as u16,
                        positions: captured_positions,
                        player: captured_color,
                    });
                }
                
                // Update capture count
                match color {
                    Color::Black => self.captures.0 += total_captured,
                    Color::White => self.captures.1 += total_captured,
                }
                
                self.pass_count = 0;
            },
            Move::Pass => {
                self.pass_count += 1;
            },
            Move::Resign => {
                // Nothing to do here, game ends
            },
        }
        
        // Record the move
        self.moves.push(mv.clone());
        
        // Emit move event
        events.insert(0, GameEvent::MoveMade {
            mv,
            by: self.current_player,
        });
        
        // Switch player
        self.current_player = self.current_player.opposite();
        
        // Check if game ended
        if self.is_game_over() {
            let (black_score, white_score) = self.calculate_score();
            events.push(GameEvent::GameFinished {
                black_score,
                white_score,
            });
        }
        
        Ok(events)
    }
    
    /// Check if the game is over
    pub fn is_game_over(&self) -> bool {
        // Game ends after two consecutive passes or resignation
        if self.pass_count >= 2 {
            return true;
        }
        
        if let Some(Move::Resign) = self.moves.last() {
            return true;
        }
        
        false
    }
    
    /// Count stones of specified color on the board
    pub fn count_stones_for(&self, color: Color) -> usize {
        self.board.iter()
            .filter(|stone| **stone == Some(color))
            .count()
    }
    
    /// Convert to Board for rules checking
    fn to_board(&self) -> Board {
        let mut board = Board::new(self.board_size);
        for y in 0..self.board_size {
            for x in 0..self.board_size {
                let idx = (y as usize) * (self.board_size as usize) + (x as usize);
                if let Some(color) = self.board[idx] {
                    board.place(Coord::new(x, y), color);
                }
            }
        }
        board
    }
    
    /// Calculate final score
    pub fn calculate_score(&self) -> (f32, f32) {
        // Basic scoring: captures + stones on board
        let black_stones = self.count_stones_for(Color::Black) as f32;
        let white_stones = self.count_stones_for(Color::White) as f32;
        let black_captures = self.captures.1 as f32; // White stones captured
        let white_captures = self.captures.0 as f32; // Black stones captured
        
        let black_score = black_stones + black_captures;
        let white_score = white_stones + white_captures + 6.5; // Komi
        
        (black_score, white_score)
    }
}

/// Game events emitted during play
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    /// A move was made
    MoveMade {
        /// The move that was made
        mv: Move,
        /// The player who made the move
        by: Color,
    },
    /// Stones were captured
    StonesCaptured {
        /// The number of stones captured
        count: u16,
        /// The coordinates of captured stones
        positions: Vec<Coord>,
        /// The player who lost the stones
        player: Color,
    },
    /// The game has ended (backward compatibility)
    GameEnded {
        /// The winner of the game (if any)
        winner: Option<Color>,
        /// The score difference
        score_diff: f32,
    },
    /// The game is finished with final score
    GameFinished {
        /// Black player score
        black_score: f32,
        /// White player score
        white_score: f32,
    },
    /// A chat message was sent
    ChatMessage {
        /// The player who sent the message
        from: Color,
        /// The message content
        message: String,
    },
}

/// Errors that can occur during game play
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GameError {
    /// The coordinate is outside the board
    #[error("Invalid coordinate")]
    InvalidCoordinate,
    
    /// The position is already occupied
    #[error("Position already occupied")]
    OccupiedPosition,
    
    /// The move violates the ko rule
    #[error("Move violates ko rule")]
    KoViolation,
    
    /// The move would result in self-capture (suicide)
    #[error("Move would result in self-capture")]
    SelfCapture,
    
    /// Other game rules violation
    #[error("Invalid move: {0}")]
    InvalidMove(String),
}

/// Simple board state representation for neural network training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardState {
    /// Board size
    pub size: u8,
    /// Board positions as 2D array
    pub board: Vec<Vec<Option<Color>>>,
}

impl BoardState {
    /// Create new empty board state
    pub fn new(size: u8) -> Self {
        Self {
            size,
            board: vec![vec![None; size as usize]; size as usize],
        }
    }
}

// Re-export CBOR types for convenience
pub use cbor::{Tag, MoveRecord};

// Re-export SGF parser for convenience
pub use sgf_parser::SGFParser;


