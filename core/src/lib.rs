// SPDX-License-Identifier: MIT OR Apache-2.0

//! P2P Go Core - Game Rules and Board Logic
//!
//! This crate provides the core game functionality including:
//! - Go board representation and manipulation
//! - Game rules and validation
//! - SGF (Smart Game Format) parsing and generation
//! - CBOR serialization helpers for game state

#![deny(unsafe_code)]
#![deny(clippy::all)]

pub mod board;
pub mod rules;
pub mod sgf;
pub mod cbor;
pub mod engine;
pub mod value_labeller;
pub mod scoring;
pub mod archiver;

use serde::{Serialize, Deserialize};
use thiserror::Error;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Move {
    /// Place a stone at the specified coordinate
    Place(Coord),
    /// Pass the turn
    Pass,
    /// Resign the game
    Resign,
}

/// Represents the current state of a Go game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
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
}

impl GameState {
    /// Create a new game with the specified board size
    pub fn new(board_size: u8) -> Self {
        let board_cells = (board_size as usize) * (board_size as usize);
        Self {
            board_size,
            board: vec![None; board_cells],
            current_player: Color::Black, // Black goes first
            moves: Vec::new(),
            pass_count: 0,
            captures: (0, 0),
        }
    }
    
    /// Apply a move to the game state
    pub fn apply_move(&mut self, mv: Move) -> Result<(), GameError> {
        // TODO: implement ko / suicide checks
        match mv {
            Move::Place(coord) => {
                if !coord.is_valid(self.board_size) {
                    return Err(GameError::InvalidCoordinate);
                }
                
                let idx = (coord.y as usize) * (self.board_size as usize) + (coord.x as usize);
                if self.board[idx].is_some() {
                    return Err(GameError::OccupiedPosition);
                }
                
                // Place the stone
                self.board[idx] = Some(self.current_player);
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
        self.moves.push(mv);
        
        // Switch player
        self.current_player = self.current_player.opposite();
        
        Ok(())
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

// Re-export CBOR types for convenience
pub use cbor::{Tag, MoveRecord};


